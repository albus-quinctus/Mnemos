//! Exportar una foto a una carpeta fuera de la biblioteca (spec 008).
//!
//! Lee el original y escribe una copia nueva en el destino: nunca toca el
//! original ni la base de datos, y nunca sobrescribe un archivo del destino.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::files::hash_file;
use crate::library::{Library, PhotoMedia};

/// Tamaño de la foto exportada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportSize {
    /// Copia exacta del archivo, con sus metadatos.
    Original,
    /// JPEG con el lado mayor de [`mnemos_imaging::EXPORT_MAX_SIDE`] como
    /// mucho, ya girado y sin metadatos.
    #[serde(rename = "reducida")]
    Reduced,
}

impl Library {
    /// Exporta la foto `id` a la carpeta `dest_dir` y devuelve la ruta del
    /// archivo creado.
    pub fn export_photo(&self, id: i64, dest_dir: &Path, size: ExportSize) -> Result<PathBuf> {
        let media = self.photo_media(id)?.ok_or(Error::PhotoNotFound(id))?;
        export(&self.root, &media, dest_dir, size)
    }
}

/// Exporta la foto `media` de la biblioteca de `root` a `dest_dir`. No
/// necesita la base de datos, así que se puede llamar sin tener la biblioteca
/// bloqueada.
pub fn export(
    root: &Path,
    media: &PhotoMedia,
    dest_dir: &Path,
    size: ExportSize,
) -> Result<PathBuf> {
    if inside(root, dest_dir)? {
        return Err(Error::DestinationInsideLibrary(dest_dir.to_owned()));
    }
    let src = media.rel_path.to_path(root);
    let name = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::NonUtf8Path(src.clone()))?;
    if !src.is_file() {
        return Err(Error::OriginalMissing(media.rel_path.to_string()));
    }
    match size {
        ExportSize::Original => {
            // Se lee una sola vez: la huella sale de lo mismo que se escribe.
            let bytes = fs::read(&src)?;
            let modified = fs::metadata(&src)?.modified()?;
            let hash = blake3::hash(&bytes).to_hex().to_string();
            write_new(dest_dir, name, |out| {
                out.write_all(&bytes)?;
                out.set_modified(modified)?;
                Ok(())
            })
            .and_then(|dest| verify(dest, &hash, &src))
        }
        ExportSize::Reduced => {
            let img = match mnemos_imaging::export_image(&src, media.orientation) {
                Ok(img) => img,
                Err(mnemos_imaging::ThumbnailError::Io(e)) => return Err(e.into()),
                Err(_) => return Err(Error::CannotReduce(media.rel_path.to_string())),
            };
            let stem = Path::new(name)
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(name);
            write_new(dest_dir, &format!("{stem}.jpg"), |out| {
                out.write_all(&img.jpeg)
            })
        }
    }
}

/// Comprueba que lo escrito en `dest` tiene la huella del original; si no,
/// lo borra.
fn verify(dest: PathBuf, hash: &str, src: &Path) -> Result<PathBuf> {
    if hash_file(&dest)? == hash {
        return Ok(dest);
    }
    let _ = fs::remove_file(&dest);
    Err(Error::CopyMismatch(src.to_owned()))
}

/// Indica si `dir` es la biblioteca de `root` o está dentro de ella.
fn inside(root: &Path, dir: &Path) -> Result<bool> {
    let root = fs::canonicalize(root)?;
    let dir = fs::canonicalize(dir)?;
    Ok(dir.starts_with(root))
}

/// Crea en `dir` un archivo nuevo con el primer nombre libre (`foto.jpg`,
/// `foto (1).jpg`…), lo llena con `fill` y lo guarda en el disco. El nombre se
/// reserva al crear el archivo, así que nunca se pisa uno existente. Si algo
/// falla, el archivo a medias se borra.
fn write_new(
    dir: &Path,
    name: &str,
    fill: impl FnOnce(&mut File) -> io::Result<()>,
) -> Result<PathBuf> {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, format!(".{ext}")),
        _ => (name, String::new()),
    };
    let mut i = 0u32;
    let (mut file, path) = loop {
        let candidate = if i == 0 {
            dir.join(name)
        } else {
            dir.join(format!("{stem} ({i}){ext}"))
        };
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (file, candidate),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => i += 1,
            Err(e) => return Err(e.into()),
        }
    };
    let written = fill(&mut file).and_then(|()| file.sync_all());
    drop(file);
    match written {
        Ok(()) => Ok(path),
        Err(e) => {
            let _ = fs::remove_file(&path);
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::ORIGINALS_DIR;

    struct Setup {
        _dir: tempfile::TempDir,
        lib: Library,
        dest: PathBuf,
        id: i64,
    }

    /// Biblioteca con una sola foto, `name`, y una carpeta de destino fuera.
    fn setup(name: &str, write: impl FnOnce(&Path)) -> Setup {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Biblioteca");
        let mut lib = Library::create(&root).unwrap();
        write(&root.join(ORIGINALS_DIR).join(name));
        lib.index().unwrap();
        let id = lib
            .connection()
            .query_row("SELECT id FROM photos", [], |r| r.get(0))
            .unwrap();
        let dest = dir.path().join("Para enviar ñ");
        fs::create_dir(&dest).unwrap();
        Setup {
            _dir: dir,
            lib,
            dest,
            id,
        }
    }

    fn jpeg(width: u32, height: u32) -> impl FnOnce(&Path) {
        move |path| {
            image::RgbImage::from_fn(width, height, |x, y| {
                image::Rgb([(x % 256) as u8, (y % 256) as u8, 90])
            })
            .save_with_format(path, image::ImageFormat::Jpeg)
            .unwrap();
        }
    }

    /// Contenido, fecha de modificación y fila de la base de datos del
    /// original, para comprobar que no cambian.
    fn snapshot(s: &Setup) -> (Vec<u8>, std::time::SystemTime, String) {
        let path = s.lib.root().join(ORIGINALS_DIR).join(
            fs::read_dir(s.lib.root().join(ORIGINALS_DIR))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .file_name(),
        );
        let row: String = s
            .lib
            .connection()
            .query_row(
                "SELECT rel_path || size_bytes || modified_at || content_hash FROM photos",
                [],
                |r| r.get(0),
            )
            .unwrap();
        (
            fs::read(&path).unwrap(),
            fs::metadata(&path).unwrap().modified().unwrap(),
            row,
        )
    }

    #[test]
    fn el_original_se_copia_identico() {
        let s = setup("Ruta del Cares.jpg", jpeg(64, 48));
        let antes = snapshot(&s);
        let out = s
            .lib
            .export_photo(s.id, &s.dest, ExportSize::Original)
            .unwrap();
        assert_eq!(out, s.dest.join("Ruta del Cares.jpg"));
        let original = s.lib.root().join("Originales/Ruta del Cares.jpg");
        assert_eq!(fs::read(&out).unwrap(), fs::read(&original).unwrap());
        assert_eq!(
            fs::metadata(&out).unwrap().modified().unwrap(),
            fs::metadata(&original).unwrap().modified().unwrap()
        );
        assert_eq!(snapshot(&s), antes);
    }

    #[test]
    fn la_reducida_es_un_jpeg_girado_y_sin_exif() {
        let s = setup("grande.png", |path| {
            image::RgbImage::from_pixel(3000, 1000, image::Rgb([200, 10, 10]))
                .save_with_format(path, image::ImageFormat::Png)
                .unwrap();
        });
        // Orientación 6: se gira 90° al exportar.
        s.lib
            .connection()
            .execute("UPDATE photos SET orientation = 6", [])
            .unwrap();
        let antes = snapshot(&s);
        let out = s
            .lib
            .export_photo(s.id, &s.dest, ExportSize::Reduced)
            .unwrap();
        assert_eq!(out, s.dest.join("grande.jpg"));
        let bytes = fs::read(&out).unwrap();
        let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg).unwrap();
        assert_eq!((img.width(), img.height()), (683, 2048));
        assert!(
            exif::Reader::new()
                .read_from_container(&mut io::Cursor::new(&bytes))
                .is_err(),
            "la reducida no debe llevar EXIF"
        );
        assert_eq!(snapshot(&s), antes);
    }

    #[test]
    fn no_sobrescribe_nada_en_el_destino() {
        let s = setup("a.jpg", jpeg(32, 32));
        fs::write(s.dest.join("a.jpg"), b"mio").unwrap();
        let primera = s
            .lib
            .export_photo(s.id, &s.dest, ExportSize::Original)
            .unwrap();
        let segunda = s
            .lib
            .export_photo(s.id, &s.dest, ExportSize::Reduced)
            .unwrap();
        assert_eq!(primera, s.dest.join("a (1).jpg"));
        assert_eq!(segunda, s.dest.join("a (2).jpg"));
        assert_eq!(fs::read(s.dest.join("a.jpg")).unwrap(), b"mio");
    }

    #[test]
    fn rechaza_exportar_dentro_de_la_biblioteca() {
        let s = setup("a.jpg", jpeg(16, 16));
        for dest in [
            s.lib.root().to_path_buf(),
            s.lib.root().join(ORIGINALS_DIR),
            s.lib.root().join(ORIGINALS_DIR).join("..").join(".mnemos"),
        ] {
            let err = s
                .lib
                .export_photo(s.id, &dest, ExportSize::Original)
                .unwrap_err();
            assert!(
                matches!(err, Error::DestinationInsideLibrary(_)),
                "{dest:?}: {err}"
            );
        }
        assert_eq!(
            fs::read_dir(s.lib.root().join(ORIGINALS_DIR))
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn un_formato_sin_decodificar_solo_sale_en_original() {
        let s = setup("movil.heic", |path| {
            fs::write(path, b"no es un heic de verdad").unwrap()
        });
        let err = s
            .lib
            .export_photo(s.id, &s.dest, ExportSize::Reduced)
            .unwrap_err();
        assert!(matches!(err, Error::CannotReduce(_)), "{err}");
        assert_eq!(fs::read_dir(&s.dest).unwrap().count(), 0);
        let out = s
            .lib
            .export_photo(s.id, &s.dest, ExportSize::Original)
            .unwrap();
        assert_eq!(fs::read(out).unwrap(), b"no es un heic de verdad");
    }

    #[test]
    fn avisa_si_falta_el_original_o_la_foto() {
        let s = setup("a.jpg", jpeg(16, 16));
        fs::remove_file(s.lib.root().join("Originales/a.jpg")).unwrap();
        let err = s
            .lib
            .export_photo(s.id, &s.dest, ExportSize::Original)
            .unwrap_err();
        assert!(matches!(err, Error::OriginalMissing(_)), "{err}");
        let err = s
            .lib
            .export_photo(999, &s.dest, ExportSize::Original)
            .unwrap_err();
        assert!(matches!(err, Error::PhotoNotFound(999)), "{err}");
        assert_eq!(fs::read_dir(&s.dest).unwrap().count(), 0);
    }

    #[test]
    fn el_tamano_llega_de_la_interfaz_en_minusculas() {
        let original: ExportSize = serde_json::from_str("\"original\"").unwrap();
        let reducida: ExportSize = serde_json::from_str("\"reducida\"").unwrap();
        assert_eq!(
            (original, reducida),
            (ExportSize::Original, ExportSize::Reduced)
        );
    }
}
