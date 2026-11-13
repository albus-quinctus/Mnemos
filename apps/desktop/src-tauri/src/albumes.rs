//! Comandos de la spec 014: álbumes. Solo cambian la base de datos; ninguno
//! toca los archivos ni borra fotos.

use mnemos_core::{Album, Error, Library};
use tauri::State;

use crate::{OpenLibrary, with_library, with_library_mut};

/// Todos los álbumes, del modificado más recientemente al más antiguo, con
/// su portada, su número de fotos y su rango de fechas.
#[tauri::command]
pub fn albumes(estado: State<'_, OpenLibrary>) -> Result<Vec<Album>, String> {
    with_library(&estado, Library::albums)
}

/// Un álbum, para la cabecera de su vista.
#[tauri::command]
pub fn album(id: i64, estado: State<'_, OpenLibrary>) -> Result<Album, String> {
    with_library(&estado, |lib| {
        lib.album(id)?.ok_or(Error::AlbumNotFound(id))
    })
}

/// Crea un álbum y le añade las fotos `ids` (vacío si no hay ninguna).
/// Devuelve el álbum ya con sus fotos.
#[tauri::command]
pub fn crear_album(
    nombre: String,
    ids: Vec<i64>,
    estado: State<'_, OpenLibrary>,
) -> Result<Album, String> {
    with_library_mut(&estado, |lib| {
        let id = lib.create_album(&nombre)?;
        lib.add_to_album(id, &ids)?;
        lib.album(id)?.ok_or(Error::AlbumNotFound(id))
    })
}

/// Cambia el nombre de un álbum. Devuelve el nombre tal como ha quedado.
#[tauri::command]
pub fn renombrar_album(
    id: i64,
    nombre: String,
    estado: State<'_, OpenLibrary>,
) -> Result<String, String> {
    with_library_mut(&estado, |lib| lib.rename_album(id, &nombre))
}

/// Elimina un álbum. Sus fotos siguen en la biblioteca.
#[tauri::command]
pub fn eliminar_album(id: i64, estado: State<'_, OpenLibrary>) -> Result<(), String> {
    with_library_mut(&estado, |lib| lib.delete_album(id))
}

/// Añade fotos a un álbum. Devuelve las que no estaban.
#[tauri::command]
pub fn anadir_a_album(
    album: i64,
    ids: Vec<i64>,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<i64>, String> {
    with_library_mut(&estado, |lib| lib.add_to_album(album, &ids))
}

/// Quita fotos de un álbum (no de la biblioteca). Devuelve las que estaban,
/// para «Deshacer».
#[tauri::command]
pub fn quitar_de_album(
    album: i64,
    ids: Vec<i64>,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<i64>, String> {
    with_library_mut(&estado, |lib| lib.remove_from_album(album, &ids))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_album_viaja_con_los_nombres_de_biblioteca_ts() {
        let album = Album {
            id: 3,
            name: "Picos".into(),
            photos: 2,
            cover: Some(9),
            cover_edit_version: Some(1),
            first: Some(10),
            last: Some(20),
            updated_at: 30,
        };
        let json = serde_json::to_value(album).unwrap();
        for campo in [
            "id",
            "name",
            "photos",
            "cover",
            "cover_edit_version",
            "first",
            "last",
            "updated_at",
        ] {
            assert!(json.get(campo).is_some(), "falta {campo}");
        }
    }
}
