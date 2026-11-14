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
use crate::taken::{Taken, set_taken, taken_at_sql};

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
            if !entry.file_type().is_file() || !is_media(entry.path()) {
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
        // Las ediciones viven en `.mnemos/ediciones/`; la base de datos solo
        // guarda qué versión se ve, y se recalcula aquí (spec 009).
        Library::sync_edit_versions(&tx, &self.root)?;
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
    /// Ya se buscó su fecha de captura (spec 004).
    has_taken: bool,
}

fn known_files(conn: &Connection) -> Result<HashMap<String, Known>> {
    let mut stmt = conn.prepare(
        "SELECT rel_path, size_bytes, modified_at, orientation IS NOT NULL,
                taken_source IS NOT NULL
         FROM photos",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get(0)?,
            Known {
                size: r.get(1)?,
                modified: r.get(2)?,
                has_orientation: r.get(3)?,
                has_taken: r.get(4)?,
            },
        ))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Indica si el archivo es una foto o un vídeo que MNEMOS reconoce.
pub(crate) fn is_media(path: &Path) -> bool {
    mnemos_imaging::is_supported_image(path) || mnemos_imaging::is_video(path)
}

/// Lo que se guarda de un archivo además de su huella y su fecha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MediaProps {
    /// Ancho y alto tal como están guardados, sin girar.
    pub dims: Option<(u32, u32)>,
    /// Orientación EXIF (1 a 8). La de un vídeo sale de su giro.
    pub orientation: u16,
    pub video: bool,
    pub duration_ms: Option<u64>,
}

impl MediaProps {
    /// Lee la cabecera del archivo: el EXIF de una foto o el contenedor de un
    /// vídeo (spec 019).
    pub(crate) fn of_file(path: &Path) -> Self {
        if !mnemos_imaging::is_video(path) {
            return MediaProps {
                dims: mnemos_imaging::dimensions(path),
                orientation: mnemos_imaging::orientation(path).unwrap_or(1),
                video: false,
                duration_ms: None,
            };
        }
        let info = mnemos_imaging::video_info(path).unwrap_or_default();
        // El giro se guarda como la orientación EXIF equivalente, así la
        // cuadrícula pone el vídeo en vertical igual que una foto.
        let orientation = match info.rotation {
            90 => 6,
            180 => 3,
            270 => 8,
            _ => 1,
        };
        let dims = info.width.zip(info.height).map(|(w, h)| {
            if mnemos_imaging::swaps_dimensions(orientation) {
                (h, w)
            } else {
                (w, h)
            }
        });
        MediaProps {
            dims,
            orientation,
            video: true,
            duration_ms: info.duration_ms,
        }
    }

    /// Una foto sin girar de este tamaño (las copias editadas, spec 009).
    pub(crate) fn photo(width: u32, height: u32) -> Self {
        MediaProps {
            dims: Some((width, height)),
            orientation: 1,
            video: false,
            duration_ms: None,
        }
    }

    fn kind(&self) -> &'static str {
        if self.video { "video" } else { "foto" }
    }
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
                params![rel.as_str(), MediaProps::of_file(path).orientation],
            )?;
        }
        if !known.has_taken {
            // Registrada antes de la migración 0006.
            set_taken(conn, rel.as_str(), &Taken::of_file(path))?;
        }
        return Ok(Outcome::Unchanged);
    }

    let hash = hash_file(path)?;
    let props = MediaProps::of_file(path);
    let taken = Taken::of_file(path);
    if known.is_some() {
        conn.execute(
            "UPDATE photos
             SET size_bytes = ?2, modified_at = ?3, content_hash = ?4, width = ?5, height = ?6,
                 orientation = ?7, media_kind = ?8, duration_ms = ?9
             WHERE rel_path = ?1",
            params![
                rel.as_str(),
                size,
                modified,
                hash,
                props.dims.map(|d| d.0),
                props.dims.map(|d| d.1),
                props.orientation,
                props.kind(),
                props.duration_ms.and_then(|d| i64::try_from(d).ok())
            ],
        )?;
        set_taken(conn, rel.as_str(), &taken)?;
        Ok(Outcome::Updated)
    } else {
        let photo = NewPhoto {
            size,
            modified,
            hash: &hash,
            props,
            taken,
        };
        insert_photo(conn, rel, &photo)?;
        Ok(Outcome::Added)
    }
}

/// Lo que se guarda de una foto nueva.
pub(crate) struct NewPhoto<'a> {
    pub size: i64,
    pub modified: i64,
    pub hash: &'a str,
    pub props: MediaProps,
    pub taken: Taken,
}

/// Registra una foto nueva y la añade al índice de búsqueda.
pub(crate) fn insert_photo(conn: &Connection, rel: &RelPath, photo: &NewPhoto) -> Result<()> {
    let NewPhoto {
        size,
        modified,
        hash,
        props,
        ref taken,
    } = *photo;
    let (exact, local) = taken.sql_params();
    conn.execute(
        &format!(
            "INSERT INTO photos
                 (rel_path, size_bytes, modified_at, content_hash, width, height, orientation,
                  imported_at, taken_at, taken_source, media_kind, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch(), {}, ?10, ?11, ?12)",
            taken_at_sql(8, 9)
        ),
        params![
            rel.as_str(),
            size,
            modified,
            hash,
            props.dims.map(|d| d.0),
            props.dims.map(|d| d.1),
            props.orientation,
            exact,
            local,
            taken.source.as_str(),
            props.kind(),
            props.duration_ms.and_then(|d| i64::try_from(d).ok())
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
    fn una_foto_movida_se_encuentra_por_su_ruta_nueva() {
        let (dir, mut lib) = library();
        let vieja = dir.path().join("Originales/Varios/a.png");
        write_png(&vieja, 1, 1, b"a");
        lib.index().unwrap();
        let nueva = dir.path().join("Originales/Vacaciones/a.png");
        fs::create_dir_all(nueva.parent().unwrap()).unwrap();
        fs::rename(&vieja, &nueva).unwrap();
        lib.index().unwrap();

        let rutas = |palabra: &str| -> Vec<String> {
            let mut stmt = lib
                .connection()
                .prepare(
                    "SELECT p.rel_path FROM photo_search s JOIN photos p ON p.id = s.rowid
                     WHERE photo_search MATCH ?1",
                )
                .unwrap();
            stmt.query_map([palabra], |r| r.get(0))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap()
        };
        assert_eq!(rutas("vacaciones"), ["Originales/Vacaciones/a.png"]);
        // La ruta vieja sigue como foto desaparecida, igual que en la cuadrícula.
        assert_eq!(rutas("varios"), ["Originales/Varios/a.png"]);
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

    #[test]
    fn rellena_la_fecha_de_las_fotos_registradas_sin_ella() {
        let (dir, mut lib) = library();
        let originals = dir.path().join("Originales");
        write_png(
            &originals.join("2026/10/IMG_20190512_143000.png"),
            1,
            1,
            b"a",
        );
        fs::write(
            originals.join("2026/10/camara.jpg"),
            include_bytes!("../../../tests/fixtures/fecha-exif.jpg"),
        )
        .unwrap();
        lib.index().unwrap();
        // Como si se hubieran registrado antes de la migración 0006.
        lib.connection()
            .execute("UPDATE photos SET taken_at = NULL, taken_source = NULL", [])
            .unwrap();

        let report = lib.index().unwrap();
        assert_eq!(report.unchanged, 2);
        let mut stmt = lib
            .connection()
            .prepare(
                "SELECT strftime('%Y-%m-%d %H:%M:%S', taken_at, 'unixepoch', 'localtime'),
                        taken_source
                 FROM photos ORDER BY rel_path",
            )
            .unwrap();
        let fechas: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            fechas,
            [
                ("2019-05-12 14:30:00".to_owned(), "nombre".to_owned()),
                ("2015-06-01 10:20:30".to_owned(), "exif".to_owned()),
            ]
        );
    }

    #[test]
    fn no_pisa_una_fecha_a_mano_aunque_el_archivo_cambie() {
        let (dir, mut lib) = library();
        let foto = dir.path().join("Originales/IMG_20190512_143000.png");
        write_png(&foto, 1, 1, b"a");
        lib.index().unwrap();
        let id: i64 = lib
            .connection()
            .query_row("SELECT id FROM photos", [], |r| r.get(0))
            .unwrap();
        lib.set_manual_taken(id, "2001-01-01 12:00").unwrap();

        // Otro programa cambia el archivo (sin pasar por MNEMOS).
        write_png(&foto, 2, 2, b"b");
        set_mtime(&foto, 1_800_000_000);
        assert_eq!(lib.index().unwrap().updated, 1);
        let fila: (String, String, String) = lib
            .connection()
            .query_row(
                "SELECT strftime('%Y-%m-%d %H:%M', taken_at, 'unixepoch', 'localtime'),
                        taken_source, taken_auto_source
                 FROM photos",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            fila,
            ("2001-01-01 12:00".into(), "manual".into(), "nombre".into())
        );
    }
}
