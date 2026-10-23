//! Núcleo de MNEMOS.
//!
//! Gestiona la biblioteca portátil: su estructura en disco, la base de datos
//! SQLite y las rutas relativas que permiten mover la biblioteca entre equipos.

pub mod db;
pub mod error;
pub mod library;
pub mod paths;

pub use error::{Error, Result};
pub use library::{Library, LibraryInfo};
pub use paths::RelPath;
