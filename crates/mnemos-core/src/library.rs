use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::db;
use crate::error::{Error, Result};

/// Carpeta con las fotos originales. MNEMOS nunca modifica su contenido.
pub const ORIGINALS_DIR: &str = "Originales";
/// Carpeta interna con la base de datos y los datos generados.
pub const DATA_DIR: &str = ".mnemos";
const DB_FILE: &str = "mnemos.db";
const DATA_SUBDIRS: &[&str] = &["miniaturas", "ediciones", "copias"];

/// Biblioteca de MNEMOS abierta.
pub struct Library {
    root: PathBuf,
    conn: Connection,
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
