use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::db;
use crate::error::{Error, Result};
use crate::paths::RelPath;

/// Carpeta con las fotos originales. MNEMOS nunca modifica su contenido.
pub const ORIGINALS_DIR: &str = "Originales";
/// Carpeta interna con la base de datos y los datos generados.
pub const DATA_DIR: &str = ".mnemos";
pub(crate) const DB_FILE: &str = "mnemos.db";
/// Carpeta de escrituras a medias dentro de `.mnemos/`: lo que hay aquí
/// nunca está en su sitio definitivo.
pub(crate) const TMP_DIR: &str = "tmp";
const DATA_SUBDIRS: &[&str] = &["miniaturas", "ediciones", "copias"];

/// Biblioteca de MNEMOS abierta.
pub struct Library {
    pub(crate) root: PathBuf,
    pub(crate) conn: Connection,
}

/// Resumen de una biblioteca para mostrar al usuario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LibraryInfo {
    pub root: String,
    pub schema_version: u32,
    pub photos: u64,
    pub albums: u64,
    pub tags: u64,
}

/// Lo que hace falta para mostrar una foto: dónde está y cómo girarla.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoMedia {
    pub rel_path: RelPath,
    pub content_hash: Option<String>,
    pub orientation: u16,
}

/// Lo que muestra el panel de detalles del visor (spec 002). Las dimensiones
/// ya tienen en cuenta la orientación.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PhotoDetails {
    pub rel_path: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub size_bytes: u64,
    pub modified_at: i64,
    /// Formato según la extensión, en mayúsculas: "JPEG", "PNG", "DNG"…
    pub format: String,
}

impl Library {
    /// Crea una biblioteca vacía en `root`. La carpeta puede existir y tener
    /// fotos, pero no puede contener ya una biblioteca de MNEMOS.
    pub fn create(root: &Path) -> Result<Self> {
        let data = root.join(DATA_DIR);
        if data.join(DB_FILE).exists() {
            return Err(Error::AlreadyALibrary(root.to_owned()));
        }
        fs::create_dir_all(root.join(ORIGINALS_DIR))?;
        for sub in DATA_SUBDIRS {
            fs::create_dir_all(data.join(sub))?;
        }
        let conn = db::open(&data.join(DB_FILE))?;
        Ok(Self {
            root: root.to_owned(),
            conn,
        })
    }

    /// Abre una biblioteca existente y actualiza su esquema si hace falta.
    pub fn open(root: &Path) -> Result<Self> {
        let db_path = root.join(DATA_DIR).join(DB_FILE);
        if !db_path.is_file() {
            return Err(Error::NotALibrary(root.to_owned()));
        }
        let conn = db::open(&db_path)?;
        Ok(Self {
            root: root.to_owned(),
            conn,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Datos para mostrar la foto `id`, o `None` si no existe.
    pub fn photo_media(&self, id: i64) -> Result<Option<PhotoMedia>> {
        let row = self
            .conn
            .query_row(
                "SELECT rel_path, content_hash, coalesce(orientation, 1) FROM photos WHERE id = ?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, u16>(2)?,
                    ))
                },
            )
            .optional()?;
        row.map(|(rel_path, content_hash, orientation)| {
            Ok(PhotoMedia {
                rel_path: RelPath::new(&rel_path)?,
                content_hash,
                orientation,
            })
        })
        .transpose()
    }

    /// Detalles de la foto `id` para el visor, o `None` si no existe.
    pub fn photo_details(&self, id: i64) -> Result<Option<PhotoDetails>> {
        let row = self
            .conn
            .query_row(
                "SELECT rel_path, width, height, size_bytes, modified_at, coalesce(orientation, 1)
                 FROM photos WHERE id = ?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<u32>>(1)?,
                        r.get::<_, Option<u32>>(2)?,
                        r.get::<_, u64>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, u16>(5)?,
                    ))
                },
            )
            .optional()?;
        Ok(
            row.map(|(rel_path, w, h, size_bytes, modified_at, orientation)| {
                let (width, height) = if mnemos_imaging::swaps_dimensions(orientation) {
                    (h, w)
                } else {
                    (w, h)
                };
                PhotoDetails {
                    format: format_name(&rel_path),
                    rel_path,
                    width,
                    height,
                    size_bytes,
                    modified_at,
                }
            }),
        )
    }

    pub fn info(&self) -> Result<LibraryInfo> {
        let count = |table: &str| -> Result<u64> {
            let sql = format!("SELECT count(*) FROM {table}");
            Ok(self.conn.query_row(&sql, [], |r| r.get(0))?)
        };
        Ok(LibraryInfo {
            root: self.root.display().to_string(),
            schema_version: db::schema_version(&self.conn)?,
            photos: count("photos")?,
            albums: count("albums")?,
            tags: count("tags")?,
        })
    }
}

/// Nombre del formato a partir de la extensión.
fn format_name(rel_path: &str) -> String {
    let ext = rel_path
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "jpg" | "jpeg" | "jpe" => "JPEG".to_owned(),
        "tif" | "tiff" => "TIFF".to_owned(),
        _ => ext.to_ascii_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crear_y_abrir_una_biblioteca() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Biblioteca");

        let lib = Library::create(&root).unwrap();
        assert!(root.join("Originales").is_dir());
        assert!(root.join(".mnemos/miniaturas").is_dir());
        assert!(root.join(".mnemos/ediciones").is_dir());
        drop(lib);

        let info = Library::open(&root).unwrap().info().unwrap();
        assert_eq!(info.schema_version, db::SCHEMA_VERSION);
        assert_eq!((info.photos, info.albums, info.tags), (0, 0, 0));
    }

    #[test]
    fn busca_una_foto_por_id() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        fs::write(dir.path().join("Originales/a.png"), b"\x89PNG\r\n\x1a\n").unwrap();
        lib.index().unwrap();
        let id: i64 = lib
            .connection()
            .query_row("SELECT id FROM photos", [], |r| r.get(0))
            .unwrap();
        let media = lib.photo_media(id).unwrap().unwrap();
        assert_eq!(media.rel_path.as_str(), "Originales/a.png");
        assert_eq!(media.orientation, 1);
        assert!(media.content_hash.is_some());
        assert_eq!(lib.photo_media(id + 1).unwrap(), None);
    }

    #[test]
    fn detalles_de_una_foto() {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        lib.connection()
            .execute(
                "INSERT INTO photos (rel_path, size_bytes, modified_at, width, height, orientation,
                                     imported_at)
                 VALUES ('Originales/2026/10/IMG_1.JPG', 4200000, 1791892800, 4000, 3000, 6, 0)",
                [],
            )
            .unwrap();
        let id = lib.connection().last_insert_rowid();
        let details = lib.photo_details(id).unwrap().unwrap();
        assert_eq!(
            details,
            PhotoDetails {
                rel_path: "Originales/2026/10/IMG_1.JPG".to_owned(),
                // Orientación 6: la foto se muestra en vertical.
                width: Some(3000),
                height: Some(4000),
                size_bytes: 4_200_000,
                modified_at: 1_791_892_800,
                format: "JPEG".to_owned(),
            }
        );
        assert_eq!(lib.photo_details(id + 1).unwrap(), None);
    }

    #[test]
    fn nombre_del_formato() {
        assert_eq!(format_name("a.jpeg"), "JPEG");
        assert_eq!(format_name("a.tif"), "TIFF");
        assert_eq!(format_name("Originales/b.dng"), "DNG");
        assert_eq!(format_name("sin_extension"), "");
    }

    #[test]
    fn no_sobrescribe_una_biblioteca_existente() {
        let dir = tempfile::tempdir().unwrap();
        Library::create(dir.path()).unwrap();
        assert!(matches!(
            Library::create(dir.path()),
            Err(Error::AlreadyALibrary(_))
        ));
    }

    #[test]
    fn crear_no_toca_fotos_existentes() {
        let dir = tempfile::tempdir().unwrap();
        let foto = dir.path().join("Originales/2024/foto.jpg");
        fs::create_dir_all(foto.parent().unwrap()).unwrap();
        fs::write(&foto, b"original").unwrap();

        Library::create(dir.path()).unwrap();
        assert_eq!(fs::read(&foto).unwrap(), b"original");
    }

    #[test]
    fn abrir_una_carpeta_cualquiera_falla() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            Library::open(dir.path()),
            Err(Error::NotALibrary(_))
        ));
    }
}
