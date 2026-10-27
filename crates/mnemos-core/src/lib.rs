//! Núcleo de MNEMOS.
//!
//! Gestiona la biblioteca portátil: su estructura en disco, la base de datos
//! SQLite y las rutas relativas que permiten mover la biblioteca entre equipos.

pub mod db;
pub mod error;
mod files;
pub mod import;
pub mod index;
pub mod library;
pub mod paths;
pub mod search;
mod taken;
pub mod thumbnails;
pub mod timeline;

pub use error::{Error, Result};
pub use import::{ImportReport, Progreso};
pub use index::{FileError, IndexReport};
pub use library::{Library, LibraryInfo, PhotoDetails, PhotoMedia};
pub use paths::RelPath;
pub use search::{DateFilter, SearchQuery};
pub use thumbnails::{ThumbnailEvent, ThumbnailJob, ThumbnailQueue, ThumbnailReport};
pub use timeline::{GridPhoto, Month, TimelineCursor, TimelinePage};
