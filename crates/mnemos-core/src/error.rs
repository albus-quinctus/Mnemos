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

    #[error("no se puede leer la carpeta de origen {path}: {source}")]
    SourceUnreadable {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("no se puede importar desde {0}: está dentro de la biblioteca")]
    SourceInsideLibrary(PathBuf),

    #[error("fecha no válida: {0}. Usa AAAA-MM-DD hh:mm, entre 1900 y 2200")]
    InvalidDate(String),

    #[error("no hay ninguna foto con el id {0}")]
    PhotoNotFound(i64),

    #[error("la miniatura del vídeo no es un JPEG del tamaño de una miniatura")]
    InvalidThumbnail,

    #[error("el archivo {0} no está en el disco")]
    OriginalMissing(String),

    #[error("no se puede exportar en {0}: está dentro de la biblioteca; elige una carpeta fuera")]
    DestinationInsideLibrary(PathBuf),

    #[error("{0} solo se puede exportar en tamaño original")]
    CannotReduce(String),

    #[error("{0} es un vídeo: los vídeos solo se exportan en tamaño original")]
    VideoNotReducible(String),

    #[error("la biblioteca usa el esquema {found}, más nuevo que el soportado ({supported})")]
    SchemaTooNew { found: u32, supported: u32 },

    #[error("«{0}» no es un día válido; se espera AAAA-MM-DD")]
    InvalidDay(String),

    #[error("no hay ningún álbum con el id {0}")]
    AlbumNotFound(i64),

    #[error("el nombre del álbum no puede estar vacío")]
    EmptyAlbumName,

    #[error("no se puede generar la copia editada de {path}: {message}")]
    CannotRender { path: String, message: String },

    #[error("«{0}» no vale como nombre de etiqueta: debe tener entre 1 y 50 caracteres")]
    InvalidTagName(String),

    #[error("no hay ninguna etiqueta con el id {0}")]
    TagNotFound(i64),

    #[error("ya existe una etiqueta llamada «{name}»")]
    TagNameTaken { name: String, existing: i64 },

    #[error("receta de edición: {0}")]
    Recipe(#[from] mnemos_imaging::recipe::RecipeError),
}

pub type Result<T> = std::result::Result<T, Error>;
