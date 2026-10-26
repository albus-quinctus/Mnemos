//! Indexado de `Originales/`: registra las fotos que hay en el disco.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use walkdir::WalkDir;

use crate::error::Result;
use crate::files::{hash_file, is_hidden, unix_secs};
use crate::library::{Library, ORIGINALS_DIR};
use crate::paths::RelPath;

/// Resultado de indexar la biblioteca.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct IndexReport {
    pub added: u64,
    pub updated: u64,
    pub unchanged: u64,
    /// Fotos registradas que ya no están en el disco. Su registro se conserva.
    pub missing: u64,
    pub errors: Vec<FileError>,
}

/// Archivo que no se pudo procesar. No detiene la operación.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileError {
    pub path: String,
    pub message: String,
}

impl FileError {
    pub(crate) fn new(path: &Path, err: impl std::fmt::Display) -> Self {
        Self {
            path: path.display().to_string(),
            message: err.to_string(),
        }
    }
}

enum Outcome {
    Added,
    Updated,
    Unchanged,
}

impl Library {
    /// Recorre `Originales/` y pone al día la base de datos.
    pub fn index(&mut self) -> Result<IndexReport> {
        let known = known_files(&self.conn)?;
        let mut seen = HashSet::new();
        let mut report = IndexReport::default();

        let tx = self.conn.transaction()?;
        let walker = WalkDir::new(self.root.join(ORIGINALS_DIR))
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_hidden(e));
        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    let path = err.path().unwrap_or(&self.root).to_owned();
                    report.errors.push(FileError::new(&path, err));
                    continue;
                }
            };
            if !entry.file_type().is_file() || !mnemos_imaging::is_supported_image(entry.path()) {
                continue;
            }
            let rel = match RelPath::from_absolute(&self.root, entry.path()) {
                Ok(rel) => rel,
                Err(err) => {
                    report.errors.push(FileError::new(entry.path(), err));
                    continue;
                }
            };
            match index_file(&tx, entry.path(), &rel, known.get(rel.as_str())) {
                Ok(Outcome::Added) => report.added += 1,
                Ok(Outcome::Updated) => report.updated += 1,
                Ok(Outcome::Unchanged) => report.unchanged += 1,
                Err(err) => report.errors.push(FileError::new(entry.path(), err)),
            }
            seen.insert(rel.as_str().to_owned());
        }
        tx.commit()?;

        report.missing = known.keys().filter(|k| !seen.contains(*k)).count() as u64;
        Ok(report)
    }
}

/// Lo que el indexado necesita saber de una foto ya registrada.
#[derive(Debug, PartialEq, Eq)]
struct Known {
    size: i64,
    modified: i64,
    has_orientation: bool,
}

fn known_files(conn: &Connection) -> Result<HashMap<String, Known>> {
    let mut stmt = conn
        .prepare("SELECT rel_path, size_bytes, modified_at, orientation IS NOT NULL FROM photos")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get(0)?,
            Known {
                size: r.get(1)?,
                modified: r.get(2)?,
                has_orientation: r.get(3)?,
            },
        ))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Orientación EXIF que se guarda: 1 (sin girar) si no se puede leer.
pub(crate) fn read_orientation(path: &Path) -> u16 {
    mnemos_imaging::orientation(path).unwrap_or(1)
}

fn index_file(
    conn: &Connection,
    path: &Path,
    rel: &RelPath,
    known: Option<&Known>,
) -> Result<Outcome> {
    let meta = fs::metadata(path)?;
    let size = meta.len() as i64;
    let modified = unix_secs(meta.modified()?);
    if let Some(known) = known.filter(|k| (k.size, k.modified) == (size, modified)) {
        if !known.has_orientation {
            // Registrada antes de la migración 0002: basta con leer el EXIF.
            conn.execute(
                "UPDATE photos SET orientation = ?2 WHERE rel_path = ?1",
                params![rel.as_str(), read_orientation(path)],
            )?;
        }
        return Ok(Outcome::Unchanged);
    }

    let hash = hash_file(path)?;
    let dims = mnemos_imaging::dimensions(path);
    let orientation = read_orientation(path);
    if known.is_some() {
        conn.execute(
            "UPDATE photos
             SET size_bytes = ?2, modified_at = ?3, content_hash = ?4, width = ?5, height = ?6,
                 orientation = ?7
             WHERE rel_path = ?1",
            params![
                rel.as_str(),
                size,
                modified,
                hash,
                dims.map(|d| d.0),
                dims.map(|d| d.1),
                orientation
            ],
        )?;
        Ok(Outcome::Updated)
    } else {
        insert_photo(conn, rel, size, modified, &hash, dims, orientation)?;
        Ok(Outcome::Added)
    }
}

/// Registra una foto nueva y la añade al índice de búsqueda.
pub(crate) fn insert_photo(
    conn: &Connection,
    rel: &RelPath,
    size: i64,
    modified: i64,
    hash: &str,
    dims: Option<(u32, u32)>,
    orientation: u16,
) -> Result<()> {
    conn.execute(
        "INSERT INTO photos
             (rel_path, size_bytes, modified_at, content_hash, width, height, orientation,
              imported_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch())",
        params![
            rel.as_str(),
            size,
            modified,
            hash,
            dims.map(|d| d.0),
            dims.map(|d| d.1),
            orientation
        ],
    )?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO photo_search (rowid, rel_path, tags) VALUES (?1, ?2, '')",
        params![id, rel.as_str()],
    )?;
    Ok(())
}

/// Indica si ya hay una foto con esta huella.
pub(crate) fn hash_exists(conn: &Connection, hash: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM photos WHERE content_hash = ?1 LIMIT 1",
            [hash],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

#[cfg(test)]
pub(crate) mod tests {
    use std::fs::File;
    use std::time::{Duration, SystemTime};

    use super::*;

    /// Escribe una imagen PNG mínima (solo cabecera) con contenido distinguible.
    pub(crate) fn write_png(path: &Path, width: u32, height: u32, salt: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
        v.extend_from_slice(&13u32.to_be_bytes());
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&width.to_be_bytes());
        v.extend_from_slice(&height.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        v.extend_from_slice(salt);
        fs::write(path, v).unwrap();
    }

    pub(crate) fn set_mtime(path: &Path, secs: u64) {
        File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
            .unwrap();
    }

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        (dir, lib)
    }

    #[test]
    fn registra_imagenes_e_ignora_lo_demas() {
        let (dir, mut lib) = library();
        let originals = dir.path().join("Originales");
        write_png(&originals.join("2024/05/playa.png"), 640, 480, b"a");
        write_png(&originals.join("raiz.PNG"), 10, 20, b"b");
        fs::write(originals.join("notas.txt"), "hola").unwrap();
        write_png(&originals.join(".oculta/x.png"), 1, 1, b"c");
        write_png(&originals.join(".y.png"), 1, 1, b"d");

        let report = lib.index().unwrap();
        assert_eq!(report.added, 2);
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        let (w, h, hash): (u32, u32, String) = lib
            .connection()
            .query_row(
                "SELECT width, height, content_hash FROM photos WHERE rel_path = 'Originales/2024/05/playa.png'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((w, h), (640, 480));
        assert_eq!(hash.len(), 64);

        let hits: i64 = lib
            .connection()
            .query_row(
                "SELECT count(*) FROM photo_search WHERE photo_search MATCH 'playa'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1);
    }

    #[test]
    fn indexar_dos_veces_no_cambia_nada() {
        let (dir, mut lib) = library();
        write_png(&dir.path().join("Originales/a.png"), 1, 1, b"a");
        lib.index().unwrap();

        let second = lib.index().unwrap();
        assert_eq!((second.added, second.updated, second.unchanged), (0, 0, 1));
    }

    #[test]
    fn actualiza_los_archivos_modificados() {
        let (dir, mut lib) = library();
        let foto = dir.path().join("Originales/a.png");
        write_png(&foto, 1, 1, b"a");
        set_mtime(&foto, 1_000);
        lib.index().unwrap();

        write_png(&foto, 800, 600, b"cambiada");
        set_mtime(&foto, 2_000);
        let report = lib.index().unwrap();
        assert_eq!(report.updated, 1);
        let w: u32 = lib
            .connection()
            .query_row("SELECT width FROM photos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(w, 800);
    }

    #[test]
    fn cuenta_las_desaparecidas_sin_borrarlas() {
        let (dir, mut lib) = library();
        let foto = dir.path().join("Originales/a.png");
        write_png(&foto, 1, 1, b"a");
        lib.index().unwrap();

        fs::remove_file(&foto).unwrap();
        let report = lib.index().unwrap();
        assert_eq!(report.missing, 1);
        assert_eq!(lib.info().unwrap().photos, 1);
    }

    /// JPEG mínimo con un segmento EXIF que solo lleva la orientación.
    fn write_jpeg_oriented(path: &Path, orientation: u16) {
        let mut tiff = b"II*\0".to_vec();
        tiff.extend_from_slice(&8u32.to_le_bytes());
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x0112u16.to_le_bytes());
        tiff.extend_from_slice(&3u16.to_le_bytes());
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&orientation.to_le_bytes());
        tiff.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend_from_slice(&tiff);
        let mut v = vec![0xFF, 0xD8, 0xFF, 0xE1];
        v.extend_from_slice(&(payload.len() as u16 + 2).to_be_bytes());
        v.extend_from_slice(&payload);
        v.extend_from_slice(&[0xFF, 0xD9]);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, v).unwrap();
    }

    fn orientation_of(lib: &Library, rel: &str) -> Option<u16> {
        lib.connection()
            .query_row(
                "SELECT orientation FROM photos WHERE rel_path = ?1",
                [rel],
                |r| r.get(0),
            )
            .unwrap()
    }

    #[test]
    fn guarda_la_orientacion_exif() {
        let (dir, mut lib) = library();
        write_jpeg_oriented(&dir.path().join("Originales/movil.jpg"), 6);
        write_png(&dir.path().join("Originales/sin_exif.png"), 1, 1, b"a");
        let report = lib.index().unwrap();
        assert_eq!(report.added, 2);
        assert_eq!(orientation_of(&lib, "Originales/movil.jpg"), Some(6));
        assert_eq!(orientation_of(&lib, "Originales/sin_exif.png"), Some(1));
    }

    #[test]
    fn completa_la_orientacion_de_fotos_anteriores_sin_rehacer_la_huella() {
        let (dir, mut lib) = library();
        write_jpeg_oriented(&dir.path().join("Originales/movil.jpg"), 8);
        lib.index().unwrap();
        // Simula una foto registrada antes de la migración 0002.
        lib.connection()
            .execute(
                "UPDATE photos SET orientation = NULL, content_hash = 'antigua'",
                [],
            )
            .unwrap();

        let report = lib.index().unwrap();
        assert_eq!((report.added, report.updated, report.unchanged), (0, 0, 1));
        assert_eq!(orientation_of(&lib, "Originales/movil.jpg"), Some(8));
        let hash: String = lib
            .connection()
            .query_row("SELECT content_hash FROM photos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(hash, "antigua");
    }

    #[test]
    fn indexa_los_raw() {
        let (dir, mut lib) = library();
        fs::write(dir.path().join("Originales/camara.NEF"), b"datos raw").unwrap();
        fs::write(dir.path().join("Originales/canon.cr3"), b"datos raw").unwrap();
        let report = lib.index().unwrap();
        assert_eq!(report.added, 2);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    #[test]
    fn indexa_cabeceras_rotas_sin_dimensiones() {
        let (dir, mut lib) = library();
        fs::write(dir.path().join("Originales/rota.jpg"), b"no es un jpeg").unwrap();
        let report = lib.index().unwrap();
        assert_eq!(report.added, 1);
        let w: Option<u32> = lib
            .connection()
            .query_row("SELECT width FROM photos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(w, None);
    }
}
