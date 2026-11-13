//! Núcleo de MNEMOS.
//!
//! Gestiona la biblioteca portátil: su estructura en disco, la base de datos
//! SQLite y las rutas relativas que permiten mover la biblioteca entre equipos.

mod copies;
pub mod day_titles;
pub mod db;
pub mod duplicates;
pub mod edits;
pub mod error;
pub mod export;
mod files;
pub mod import;
pub mod index;
pub mod library;
mod organize;
pub mod paths;
pub mod search;
pub mod selection;
mod taken;
mod takeout;
pub mod thumbnails;
pub mod timeline;
pub mod trash;

pub use day_titles::{DayTitle, MAX_DAY_TITLE};
pub use duplicates::{
    DuplicateCopy, DuplicateCursor, DuplicateGroup, DuplicatePage, DuplicateSummary,
};
pub use edits::{EditSummary, EditVersion, StoredRecipe};
pub use error::{Error, Result};
pub use export::ExportSize;
pub use import::{ImportReport, Progreso};
pub use index::{FileError, IndexReport};
pub use library::{Library, LibraryInfo, MediaEdit, PhotoDetails, PhotoMedia};
pub use paths::RelPath;
pub use search::{DateFilter, SearchQuery};
pub use selection::Scope;
pub use thumbnails::{ThumbnailEvent, ThumbnailJob, ThumbnailQueue, ThumbnailReport};
pub use timeline::{GridPhoto, Month, TimelineCursor, TimelinePage, View};
