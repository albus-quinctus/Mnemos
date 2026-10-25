use std::path::PathBuf;

/// Errores del núcleo de MNEMOS.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error de entrada/salida: {0}")]
    Io(#[from] std::io::Error),

    #[error("error de base de datos: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("no hay ninguna biblioteca de MNEMOS en {0}")]
    NotALibrary(PathBuf),

    #[error("ya existe una biblioteca de MNEMOS en {0}")]
    AlreadyALibrary(PathBuf),

    #[error("ruta no válida dentro de la biblioteca: {0}")]
    InvalidRelPath(String),

    #[error("la ruta {0} está fuera de la biblioteca")]
    OutsideLibrary(PathBuf),

    #[error("el nombre de {0} no es UTF-8 válido")]
    NonUtf8Path(PathBuf),

    #[error("la copia de {0} no coincide con el original; se ha descartado")]
    CopyMismatch(PathBuf),

    #[error("no se puede importar desde {0}: está dentro de la biblioteca")]
    SourceInsideLibrary(PathBuf),

    #[error("la biblioteca usa el esquema {found}, más nuevo que el soportado ({supported})")]
    SchemaTooNew { found: u32, supported: u32 },
}

pub type Result<T> = std::result::Result<T, Error>;
