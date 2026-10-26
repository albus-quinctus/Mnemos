//! Importación: copia fotos de una carpeta externa a `Originales/AAAA/MM/`.
//!
//! El origen nunca se modifica. Cada copia se verifica con su huella antes de
//! entrar en `Originales/`.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::Connection;
use serde::Serialize;
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::files::{hash_file, is_hidden, unix_secs, year_month};
use crate::index::{FileError, hash_exists, insert_photo, read_orientation};
use crate::library::{DATA_DIR, Library, ORIGINALS_DIR, TMP_DIR};
use crate::paths::RelPath;

/// Resultado de una importación.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    pub copied: u64,
    /// Fotos cuyo contenido ya estaba en la biblioteca o repetido en el origen.
    pub duplicates: u64,
    pub errors: Vec<FileError>,
    /// Si se paró a petición del usuario. Lo ya copiado se conserva.
    pub cancelled: bool,
}

/// Avance de una importación: cuántas imágenes hay en total y cuántas ya se
/// han tratado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progreso {
    pub total: u64,
    pub procesadas: u64,
}

enum Outcome {
    Copied,
    Duplicate,
}

impl Library {
    /// Copia las imágenes de `source` (y sus subcarpetas) a la biblioteca.
    pub fn import_from(&mut self, source: &Path) -> Result<ImportReport> {
        self.import_from_with(source, &AtomicBool::new(false), |_, _| {})
    }

    /// Como `import_from`, pero informa tras cada archivo y se para si `cancelar`
    /// se activa. Cada archivo se confirma por separado, así que una parada o un
    /// corte deja la biblioteca coherente con lo ya copiado.
    pub fn import_from_with(
        &mut self,
        source: &Path,
        cancelar: &AtomicBool,
        mut progreso: impl FnMut(Progreso, &ImportReport),
    ) -> Result<ImportReport> {
        let source = source.canonicalize()?;
        let root = self.root.canonicalize()?;
        if source.starts_with(&root) {
            return Err(Error::SourceInsideLibrary(source));
        }
        let tmp = self.root.join(DATA_DIR).join(TMP_DIR);
        fs::create_dir_all(&tmp)?;

        let mut report = ImportReport::default();
        let archivos = candidatos(&source, &root, &mut report);
        let total = archivos.len() as u64;
        let mut imported = HashSet::new();
        for (i, archivo) in archivos.iter().enumerate() {
            if cancelar.load(Ordering::Relaxed) {
                report.cancelled = true;
                break;
            }
            match import_file(&self.conn, &self.root, &tmp, archivo, &mut imported) {
                Ok(Outcome::Copied) => report.copied += 1,
                Ok(Outcome::Duplicate) => report.duplicates += 1,
                Err(err) => report.errors.push(FileError::new(archivo, err)),
            }
            progreso(
                Progreso {
                    total,
                    procesadas: i as u64 + 1,
                },
                &report,
            );
        }
        Ok(report)
    }
}

/// Imágenes de `source` que pueden importarse. Los errores al recorrer se
/// anotan en `report`. Si la biblioteca está dentro del origen, no se recorre.
fn candidatos(source: &Path, root: &Path, report: &mut ImportReport) -> Vec<PathBuf> {
    let walker = WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && e.path() != root);
    let mut archivos = Vec::new();
    for entry in walker {
        match entry {
            Ok(entry)
                if entry.file_type().is_file()
                    && mnemos_imaging::is_supported_image(entry.path()) =>
            {
                archivos.push(entry.into_path());
            }
            Ok(_) => {}
            Err(err) => {
                let path = err.path().unwrap_or(source).to_owned();
                report.errors.push(FileError::new(&path, err));
            }
        }
    }
    archivos
}

fn import_file(
    conn: &Connection,
    root: &Path,
    tmp: &Path,
    src: &Path,
    imported: &mut HashSet<String>,
) -> Result<Outcome> {
    let name = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::NonUtf8Path(src.to_owned()))?;
    let meta = fs::metadata(src)?;
    let modified = meta.modified()?;
    let hash = hash_file(src)?;
    if imported.contains(&hash) || hash_exists(conn, &hash)? {
        return Ok(Outcome::Duplicate);
    }

    let part = tmp.join(format!("{hash}.part"));
    if let Err(err) = copy_verified(src, &part, &hash, modified) {
        let _ = fs::remove_file(&part);
        return Err(err);
    }

    let (year, month) = year_month(unix_secs(modified));
    let dir = root
        .join(ORIGINALS_DIR)
        .join(format!("{year:04}"))
        .join(format!("{month:02}"));
    let dest = fs::create_dir_all(&dir)
        .map_err(Error::from)
        .and_then(|()| free_name(&dir, name))
        .and_then(|dest| Ok(fs::rename(&part, &dest).map(|()| dest)?));
    let dest = match dest {
        Ok(dest) => dest,
        Err(err) => {
            let _ = fs::remove_file(&part);
            return Err(err);
        }
    };

    let rel = RelPath::from_absolute(root, &dest)?;
    // La foto y su entrada en el índice de búsqueda se guardan juntas.
    let tx = conn.unchecked_transaction()?;
    insert_photo(
        &tx,
        &rel,
        meta.len() as i64,
        unix_secs(modified),
        &hash,
        mnemos_imaging::dimensions(&dest),
        read_orientation(&dest),
    )?;
    tx.commit()?;
    imported.insert(hash);
    Ok(Outcome::Copied)
}

/// Copia `src` en `dest`, conserva su fecha de modificación y comprueba que
/// la copia tiene la misma huella que el original.
fn copy_verified(
    src: &Path,
    dest: &Path,
    hash: &str,
    modified: std::time::SystemTime,
) -> Result<()> {
    let mut out = File::create(dest)?;
    io::copy(&mut File::open(src)?, &mut out)?;
    out.set_modified(modified)?;
    out.sync_all()?;
    drop(out);
    if hash_file(dest)? != hash {
        return Err(Error::CopyMismatch(src.to_owned()));
    }
    Ok(())
}

/// Primer nombre libre en `dir`: `foto.jpg`, `foto (1).jpg`, `foto (2).jpg`…
fn free_name(dir: &Path, name: &str) -> Result<PathBuf> {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return Ok(candidate);
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, format!(".{ext}")),
        _ => (name, String::new()),
    };
    for i in 1.. {
        let candidate = dir.join(format!("{stem} ({i}){ext}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    unreachable!("el rango 1.. no termina")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::tests::{set_mtime, write_png};

    // 2023-11-14 22:13:20 UTC
    const NOV_2023: u64 = 1_700_000_000;

    fn setup() -> (tempfile::TempDir, Library, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(&dir.path().join("Biblioteca")).unwrap();
        let src = dir.path().join("Camara");
        fs::create_dir_all(&src).unwrap();
        (dir, lib, src)
    }

    #[test]
    fn copia_a_anio_y_mes_sin_tocar_el_origen() {
        let (dir, mut lib, src) = setup();
        let foto = src.join("viaje/IMG_0001.png");
        write_png(&foto, 4000, 3000, b"a");
        set_mtime(&foto, NOV_2023);
        let before = fs::read(&foto).unwrap();
        let before_mtime = fs::metadata(&foto).unwrap().modified().unwrap();

        let report = lib.import_from(&src).unwrap();
        assert_eq!(report.copied, 1);
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        // El origen queda intacto.
        assert_eq!(fs::read(&foto).unwrap(), before);
        assert_eq!(
            fs::metadata(&foto).unwrap().modified().unwrap(),
            before_mtime
        );

        // La copia está en su sitio, idéntica y con la misma fecha.
        let dest = dir
            .path()
            .join("Biblioteca/Originales/2023/11/IMG_0001.png");
        assert_eq!(fs::read(&dest).unwrap(), before);
        assert_eq!(
            fs::metadata(&dest).unwrap().modified().unwrap(),
            before_mtime
        );

        // Registrada y sin restos temporales.
        let (rel, w): (String, u32) = lib
            .connection()
            .query_row("SELECT rel_path, width FROM photos", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(rel, "Originales/2023/11/IMG_0001.png");
        assert_eq!(w, 4000);
        let tmp = dir.path().join("Biblioteca/.mnemos/tmp");
        assert_eq!(fs::read_dir(tmp).unwrap().count(), 0);

        // Indexar después no encuentra nada nuevo.
        let index = lib.index().unwrap();
        assert_eq!((index.added, index.unchanged), (0, 1));
    }

    #[test]
    fn no_importa_duplicados() {
        let (_dir, mut lib, src) = setup();
        write_png(&src.join("a.png"), 1, 1, b"igual");
        write_png(&src.join("copia/a-bis.png"), 1, 1, b"igual");

        let first = lib.import_from(&src).unwrap();
        assert_eq!((first.copied, first.duplicates), (1, 1));

        let second = lib.import_from(&src).unwrap();
        assert_eq!((second.copied, second.duplicates), (0, 2));
        assert_eq!(lib.info().unwrap().photos, 1);
    }

    #[test]
    fn no_sobrescribe_nombres_repetidos() {
        let (dir, mut lib, src) = setup();
        let a = src.join("uno/foto.png");
        let b = src.join("dos/foto.png");
        write_png(&a, 1, 1, b"a");
        write_png(&b, 1, 1, b"b");
        set_mtime(&a, NOV_2023);
        set_mtime(&b, NOV_2023);

        let report = lib.import_from(&src).unwrap();
        assert_eq!(report.copied, 2);
        let month = dir.path().join("Biblioteca/Originales/2023/11");
        assert!(month.join("foto.png").is_file());
        assert!(month.join("foto (1).png").is_file());
    }

    #[test]
    fn rechaza_importar_desde_la_propia_biblioteca() {
        let (dir, mut lib, _src) = setup();
        let inside = dir.path().join("Biblioteca/Originales");
        assert!(matches!(
            lib.import_from(&inside),
            Err(Error::SourceInsideLibrary(_))
        ));
    }

    #[test]
    fn salta_la_biblioteca_si_esta_dentro_del_origen() {
        let (dir, mut lib, src) = setup();
        write_png(&src.join("a.png"), 1, 1, b"a");
        write_png(&dir.path().join("Biblioteca/Originales/b.png"), 1, 1, b"b");

        let report = lib.import_from(dir.path()).unwrap();
        assert_eq!(report.copied, 1);
    }

    #[test]
    fn informa_del_avance_y_se_para_al_cancelar() {
        let (dir, mut lib, src) = setup();
        write_png(&src.join("a.png"), 1, 1, b"a");
        write_png(&src.join("b.png"), 1, 1, b"b");
        write_png(&src.join("c.png"), 1, 1, b"c");
        let cancelar = AtomicBool::new(false);
        let mut avances = Vec::new();
        let report = lib
            .import_from_with(&src, &cancelar, |p, _| {
                avances.push(p);
                if p.procesadas == 1 {
                    cancelar.store(true, Ordering::Relaxed);
                }
            })
            .unwrap();

        assert!(report.cancelled);
        assert_eq!(report.copied, 1);
        assert_eq!(avances.len(), 1);
        assert_eq!(avances[0].total, 3);
        // Lo ya copiado queda registrado y no quedan restos temporales.
        assert_eq!(lib.info().unwrap().photos, 1);
        let tmp = dir.path().join("Biblioteca/.mnemos/tmp");
        assert_eq!(fs::read_dir(tmp).unwrap().count(), 0);
    }

    #[test]
    fn lo_importado_se_puede_buscar() {
        let (_dir, mut lib, src) = setup();
        write_png(&src.join("Cumpleaños/a.png"), 1, 1, b"a");
        write_png(&src.join("b.png"), 1, 1, b"b");
        lib.import_from(&src).unwrap();
        let (fotos, indexadas): (i64, i64) = lib
            .connection()
            .query_row(
                "SELECT (SELECT count(*) FROM photos), (SELECT count(*) FROM photo_search)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((fotos, indexadas), (2, 2));
    }

    #[test]
    fn nombres_libres() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            free_name(dir.path(), "a.jpg").unwrap(),
            dir.path().join("a.jpg")
        );
        fs::write(dir.path().join("a.jpg"), b"").unwrap();
        fs::write(dir.path().join("a (1).jpg"), b"").unwrap();
        assert_eq!(
            free_name(dir.path(), "a.jpg").unwrap(),
            dir.path().join("a (2).jpg")
        );
        fs::write(dir.path().join("sin"), b"").unwrap();
        assert_eq!(
            free_name(dir.path(), "sin").unwrap(),
            dir.path().join("sin (1)")
        );
    }
}
