//! «Guardar como copia» (spec 009): la foto editada como una foto nueva e
//! independiente, junto al original.
//!
//! El original no se toca. La copia se escribe entera en `.mnemos/tmp/` y
//! solo entonces pasa a la carpeta del original con un nombre libre; si algo
//! falla, no queda ni archivo a medias ni registro en la base de datos.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use mnemos_imaging::recipe::Geometry;
use mnemos_imaging::{CopyFormat, EditedCopy, ThumbnailError};
use rusqlite::{OptionalExtension, params};

use crate::error::{Error, Result};
use crate::files::hash_file;
use crate::import::numbered_name;
use crate::index::{MediaProps, NewPhoto, insert_photo};
use crate::library::{DATA_DIR, Library, TMP_DIR};
use crate::paths::RelPath;
use crate::taken::Taken;

/// Datos del original que pasan a la copia, además de la fecha de
/// modificación del archivo. No pasan los álbumes ni el archivado.
const COPIED_COLUMNS: &str = "taken_at, taken_source, taken_auto_at, taken_auto_source,
     description, latitude, longitude, altitude, favorite";

impl Library {
    /// Guarda la foto `id` con `geometry` aplicada como una foto nueva en la
    /// misma carpeta: `IMG_1234 (1).jpg`, o `(2)`, `(3)`… si ya existe.
    /// Devuelve el id de la copia.
    ///
    /// La copia sale en JPEG de calidad 95 a tamaño completo, o en PNG si el
    /// original es PNG, con el EXIF del original (orientación 1 y las
    /// dimensiones nuevas). Toma del original la fecha de captura y su
    /// origen, la fecha de modificación, la descripción, la ubicación y el
    /// favorito. El original y su receta no cambian.
    pub fn save_edit_as_copy(&mut self, id: i64, geometry: &Geometry) -> Result<i64> {
        geometry.check()?;
        let (rel_path, orientation, modified) = self
            .conn
            .query_row(
                "SELECT rel_path, coalesce(orientation, 1), modified_at FROM photos WHERE id = ?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, u16>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or(Error::PhotoNotFound(id))?;
        let rel = RelPath::new(&rel_path)?;
        let src = rel.to_path(&self.root);
        if !src.is_file() {
            return Err(Error::OriginalMissing(rel_path));
        }
        let copy =
            mnemos_imaging::edited_copy(&src, orientation, geometry).map_err(|err| match err {
                ThumbnailError::Io(err) => Error::Io(err),
                other => Error::CannotRender {
                    path: rel_path.clone(),
                    message: other.to_string(),
                },
            })?;
        let (stem, ext) = copy_name(&src, copy.format)?;
        let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(modified.max(0).unsigned_abs());

        let part = write_part(&self.root, &copy.bytes, mtime)?;
        // `src` está dentro de `Originales/`, así que siempre tiene carpeta.
        let dir = src.parent().unwrap_or(&self.root);
        let dest = numbered_name(dir, &stem, &ext);
        if let Err(err) = fs::rename(&part, &dest) {
            let _ = fs::remove_file(&part);
            return Err(err.into());
        }
        match self.register_copy(id, &dest, &copy, modified) {
            Ok(copy_id) => Ok(copy_id),
            Err(err) => {
                // Es el archivo recién creado, no un original.
                let _ = fs::remove_file(&dest);
                Err(err)
            }
        }
    }

    /// Indexa la copia ya en su sitio y le pasa los datos del original, todo
    /// en una transacción.
    fn register_copy(
        &mut self,
        original_id: i64,
        dest: &Path,
        copy: &EditedCopy,
        modified: i64,
    ) -> Result<i64> {
        let rel = RelPath::from_absolute(&self.root, dest)?;
        let hash = hash_file(dest)?;
        let size = fs::metadata(dest)?.len() as i64;
        let tx = self.conn.transaction()?;
        let photo = NewPhoto {
            size,
            modified,
            hash: &hash,
            props: MediaProps::photo(copy.width, copy.height),
            taken: Taken::NONE,
        };
        insert_photo(&tx, &rel, &photo)?;
        let copy_id: i64 = tx.query_row(
            "SELECT id FROM photos WHERE rel_path = ?1",
            [rel.as_str()],
            |r| r.get(0),
        )?;
        tx.execute(
            &format!(
                "UPDATE photos SET ({COPIED_COLUMNS}) =
                     (SELECT {COPIED_COLUMNS} FROM photos WHERE id = ?2)
                 WHERE id = ?1"
            ),
            params![copy_id, original_id],
        )?;
        crate::tags::copy_tags(&tx, original_id, copy_id)?;
        tx.commit()?;
        Ok(copy_id)
    }
}

/// Nombre y extensión de la copia: los del original si el formato no
/// cambia (`IMG_1234.JPG` sigue en `.JPG`), o la extensión del formato de la
/// copia (`IMG_1234.CR2` pasa a `.jpg`).
pub(crate) fn copy_name(src: &Path, format: CopyFormat) -> Result<(String, String)> {
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error::NonUtf8Path(src.to_owned()))?;
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| format.matches_extension(e))
        .unwrap_or(format.extension());
    Ok((stem.to_owned(), ext.to_owned()))
}

/// Escribe la copia entera en `.mnemos/tmp/`, con la fecha de modificación
/// del original, y la lleva al disco. Si falla, no deja nada.
fn write_part(root: &Path, bytes: &[u8], modified: SystemTime) -> Result<PathBuf> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let tmp_dir = root.join(DATA_DIR).join(TMP_DIR);
    fs::create_dir_all(&tmp_dir)?;
    let part = tmp_dir.join(format!(
        "copia-{}-{}.part",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let written = File::create(&part).and_then(|mut file| {
        file.write_all(bytes)?;
        file.set_modified(modified)?;
        file.sync_all()
    });
    match written {
        Ok(()) => Ok(part),
        Err(err) => {
            let _ = fs::remove_file(&part);
            Err(err.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use mnemos_imaging::recipe::Crop;

    use super::*;
    use crate::edits::recipe_rel_path;
    use crate::files::unix_secs;
    use crate::index::tests::set_mtime;
    use crate::library::ORIGINALS_DIR;

    // 2023-11-14 22:13:20 UTC
    const NOV_2023: u64 = 1_700_000_000;

    struct Setup {
        dir: tempfile::TempDir,
        lib: Library,
        id: i64,
    }

    impl Setup {
        fn folder(&self) -> PathBuf {
            self.dir.path().join(ORIGINALS_DIR).join("2021").join("07")
        }

        fn names(&self) -> Vec<String> {
            let mut names: Vec<String> = fs::read_dir(self.folder())
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            names.sort();
            names
        }

        fn count(&self) -> i64 {
            self.lib
                .connection()
                .query_row("SELECT count(*) FROM photos", [], |r| r.get(0))
                .unwrap()
        }

        fn tmp_is_empty(&self) -> bool {
            let tmp = self.dir.path().join(DATA_DIR).join(TMP_DIR);
            fs::read_dir(tmp).map_or(true, |mut d| d.next().is_none())
        }
    }

    /// Biblioteca con una foto `Originales/2021/07/<name>` de 600 × 400.
    fn setup(name: &str, format: image::ImageFormat) -> Setup {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        let folder = dir.path().join(ORIGINALS_DIR).join("2021").join("07");
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        image::RgbImage::from_fn(600, 400, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 90])
        })
        .save_with_format(&path, format)
        .unwrap();
        set_mtime(&path, NOV_2023);
        lib.index().unwrap();
        let id = lib
            .connection()
            .query_row("SELECT id FROM photos", [], |r| r.get(0))
            .unwrap();
        Setup { dir, lib, id }
    }

    fn giro() -> Geometry {
        Geometry {
            rotate: 90,
            ..Geometry::default()
        }
    }

    fn file_state(path: &Path) -> (Vec<u8>, SystemTime) {
        (
            fs::read(path).unwrap(),
            fs::metadata(path).unwrap().modified().unwrap(),
        )
    }

    type Row = (
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        bool,
    );

    fn copied_data(lib: &Library, id: i64) -> Row {
        lib.connection()
            .query_row(
                &format!("SELECT {COPIED_COLUMNS} FROM photos WHERE id = ?1"),
                [id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                    ))
                },
            )
            .unwrap()
    }

    #[test]
    fn la_copia_va_junto_al_original_con_numero_y_sus_datos() {
        let mut s = setup("IMG_1234.jpg", image::ImageFormat::Jpeg);
        s.lib
            .connection()
            .execute(
                "UPDATE photos SET taken_at = 1626287405, taken_source = 'manual',
                     taken_auto_at = 1626280000, taken_auto_source = 'exif',
                     description = 'Subida al Naranjo', latitude = 43.2, longitude = -4.8,
                     altitude = 2519.0, favorite = 1, archived_at = 1700000500
                 WHERE id = ?1",
                [s.id],
            )
            .unwrap();
        let original = s.folder().join("IMG_1234.jpg");
        let before = file_state(&original);

        let copy_id = s.lib.save_edit_as_copy(s.id, &giro()).unwrap();
        assert_ne!(copy_id, s.id);
        assert_eq!(s.names(), ["IMG_1234 (1).jpg", "IMG_1234.jpg"]);
        assert_eq!(file_state(&original), before, "el original no cambia");
        assert!(s.tmp_is_empty());

        // Indexada como otra foto, con los datos del original.
        assert_eq!(copied_data(&s.lib, copy_id), copied_data(&s.lib, s.id));
        let (rel, w, h, orientation, modified, archived, version): (
            String,
            u32,
            u32,
            u16,
            i64,
            Option<i64>,
            Option<u32>,
        ) = s
            .lib
            .connection()
            .query_row(
                "SELECT rel_path, width, height, orientation, modified_at, archived_at,
                        edit_version
                 FROM photos WHERE id = ?1",
                [copy_id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(rel, "Originales/2021/07/IMG_1234 (1).jpg");
        assert_eq!((w, h, orientation), (400, 600, 1));
        assert_eq!(modified, NOV_2023 as i64);
        assert_eq!((archived, version), (None, None));
        let copy_path = s.folder().join("IMG_1234 (1).jpg");
        assert_eq!(
            unix_secs(fs::metadata(&copy_path).unwrap().modified().unwrap()),
            NOV_2023 as i64
        );
        let img = image::open(&copy_path).unwrap();
        assert_eq!((img.width(), img.height()), (400, 600));
        // La copia ya está editada: ni el original ni ella tienen receta.
        let hash: String = s
            .lib
            .connection()
            .query_row(
                "SELECT content_hash FROM photos WHERE id = ?1",
                [s.id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!s.dir.path().join(recipe_rel_path(&hash)).exists());

        // La segunda copia toma el siguiente número.
        s.lib.save_edit_as_copy(s.id, &giro()).unwrap();
        assert_eq!(
            s.names(),
            ["IMG_1234 (1).jpg", "IMG_1234 (2).jpg", "IMG_1234.jpg"]
        );

        // Volver a indexar no cambia nada: tamaño y fecha coinciden.
        let report = s.lib.index().unwrap();
        assert_eq!((report.added, report.updated, report.unchanged), (0, 0, 3));
        assert_eq!(copied_data(&s.lib, copy_id), copied_data(&s.lib, s.id));
        assert_eq!(file_state(&original), before);
    }

    #[test]
    fn la_copia_lleva_las_etiquetas_del_original() {
        let mut s = setup("IMG_1234.jpg", image::ImageFormat::Jpeg);
        let playa = s.lib.create_tag("Playa").unwrap();
        let abuela = s.lib.create_tag("Abuela").unwrap();
        s.lib.add_tag(&[s.id], playa.id).unwrap();
        s.lib.add_tag(&[s.id], abuela.id).unwrap();

        let copy_id = s.lib.save_edit_as_copy(s.id, &giro()).unwrap();
        let nombres = |id| -> Vec<String> {
            s.lib
                .photo_tags(id)
                .unwrap()
                .into_iter()
                .map(|t| t.name)
                .collect()
        };
        assert_eq!(nombres(copy_id), ["Abuela", "Playa"]);
        let mut encontradas: Vec<i64> = s
            .lib
            .search_page(&crate::SearchQuery::parse("abuela"), None, 10)
            .unwrap()
            .photos
            .iter()
            .map(|p| p.id)
            .collect();
        encontradas.sort();
        assert_eq!(encontradas, [s.id, copy_id]);
    }

    #[test]
    fn un_png_da_un_png_y_otros_formatos_un_jpg() {
        let mut s = setup("captura.png", image::ImageFormat::Png);
        s.lib.save_edit_as_copy(s.id, &giro()).unwrap();
        assert_eq!(s.names(), ["captura (1).png", "captura.png"]);

        let mut s = setup("escaneo.bmp", image::ImageFormat::Bmp);
        s.lib.save_edit_as_copy(s.id, &giro()).unwrap();
        assert_eq!(s.names(), ["escaneo (1).jpg", "escaneo.bmp"]);
        let bytes = fs::read(s.folder().join("escaneo (1).jpg")).unwrap();
        assert!(bytes.starts_with(&[0xFF, 0xD8, 0xFF]));

        // Un JPEG con la extensión en mayúsculas la conserva.
        let mut s = setup("DSC_0001.JPG", image::ImageFormat::Jpeg);
        let crop = Geometry {
            crop: Some(Crop {
                x: 0.0,
                y: 0.0,
                width: 0.5,
                height: 0.5,
            }),
            ..Geometry::default()
        };
        s.lib.save_edit_as_copy(s.id, &crop).unwrap();
        assert_eq!(s.names(), ["DSC_0001 (1).JPG", "DSC_0001.JPG"]);
        let img = image::open(s.folder().join("DSC_0001 (1).JPG")).unwrap();
        assert_eq!((img.width(), img.height()), (300, 200));
    }

    #[test]
    fn si_no_se_puede_escribir_en_tmp_no_queda_nada() {
        let mut s = setup("IMG_1234.jpg", image::ImageFormat::Jpeg);
        let tmp = s.dir.path().join(DATA_DIR).join(TMP_DIR);
        let _ = fs::remove_dir_all(&tmp);
        fs::write(&tmp, b"no soy una carpeta").unwrap();
        let before = file_state(&s.folder().join("IMG_1234.jpg"));

        assert!(s.lib.save_edit_as_copy(s.id, &giro()).is_err());
        assert_eq!(s.names(), ["IMG_1234.jpg"]);
        assert_eq!(s.count(), 1);
        assert_eq!(file_state(&s.folder().join("IMG_1234.jpg")), before);
    }

    #[test]
    fn si_falla_el_registro_se_quita_el_archivo_nuevo() {
        let mut s = setup("IMG_1234.jpg", image::ImageFormat::Jpeg);
        s.lib
            .connection()
            .execute_batch(
                "CREATE TRIGGER sin_fotos BEFORE INSERT ON photos
                 BEGIN SELECT RAISE(ABORT, 'disco desconectado'); END;",
            )
            .unwrap();
        let before = file_state(&s.folder().join("IMG_1234.jpg"));

        let err = s.lib.save_edit_as_copy(s.id, &giro()).unwrap_err();
        assert!(err.to_string().contains("disco desconectado"), "{err}");
        assert_eq!(s.names(), ["IMG_1234.jpg"]);
        assert_eq!(s.count(), 1);
        assert!(s.tmp_is_empty());
        assert_eq!(file_state(&s.folder().join("IMG_1234.jpg")), before);
    }

    #[test]
    fn sin_original_o_con_geometria_no_valida_no_se_crea_nada() {
        let mut s = setup("IMG_1234.jpg", image::ImageFormat::Jpeg);
        let bad = Geometry {
            rotate: 45,
            ..Geometry::default()
        };
        assert!(matches!(
            s.lib.save_edit_as_copy(s.id, &bad),
            Err(Error::Recipe(_))
        ));
        assert!(matches!(
            s.lib.save_edit_as_copy(9999, &giro()),
            Err(Error::PhotoNotFound(9999))
        ));
        assert_eq!(s.names(), ["IMG_1234.jpg"]);

        // Si el original no está en el disco no hay nada que copiar.
        let moved = s.dir.path().join("fuera.jpg");
        fs::rename(s.folder().join("IMG_1234.jpg"), &moved).unwrap();
        assert!(matches!(
            s.lib.save_edit_as_copy(s.id, &giro()),
            Err(Error::OriginalMissing(_))
        ));
        assert_eq!(s.count(), 1);
    }
}
