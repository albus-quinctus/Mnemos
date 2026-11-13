//! Comandos de la spec 016: etiquetas. Solo cambian la base de datos;
//! ninguno toca los archivos.

use mnemos_core::{Error, Library, Tag, TagUsage};
use serde::Serialize;
use tauri::State;

use crate::{OpenLibrary, with_library, with_library_mut};

/// Todas las etiquetas, con su número de fotos y su portada.
#[tauri::command]
pub fn etiquetas(estado: State<'_, OpenLibrary>) -> Result<Vec<Tag>, String> {
    with_library(&estado, Library::tags)
}

/// Crea una etiqueta, o devuelve la que ya se llama así.
#[tauri::command]
pub fn crear_etiqueta(nombre: String, estado: State<'_, OpenLibrary>) -> Result<Tag, String> {
    with_library_mut(&estado, |lib| lib.create_tag(&nombre))
}

/// Resultado de renombrar: la etiqueta con su nombre nuevo o, si ya hay otra
/// con ese nombre, esa otra, para ofrecer unirlas.
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Renombrado {
    Hecho(Tag),
    Existe(Tag),
}

#[tauri::command]
pub fn renombrar_etiqueta(
    id: i64,
    nombre: String,
    estado: State<'_, OpenLibrary>,
) -> Result<Renombrado, String> {
    with_library_mut(&estado, |lib| match lib.rename_tag(id, &nombre) {
        Ok(tag) => Ok(Renombrado::Hecho(tag)),
        Err(Error::TagNameTaken { existing, .. }) => Ok(Renombrado::Existe(lib.tag(existing)?)),
        Err(e) => Err(e),
    })
}

/// Pasa las fotos de `desde` a `en` y borra `desde`.
#[tauri::command]
pub fn unir_etiquetas(desde: i64, en: i64, estado: State<'_, OpenLibrary>) -> Result<Tag, String> {
    with_library_mut(&estado, |lib| lib.merge_tags(desde, en))
}

/// Borra una etiqueta. Devuelve sus fotos, para «Deshacer».
#[tauri::command]
pub fn borrar_etiqueta(id: i64, estado: State<'_, OpenLibrary>) -> Result<Vec<i64>, String> {
    with_library_mut(&estado, |lib| lib.delete_tag(id))
}

/// Pone una etiqueta a varias fotos. Devuelve las que no la tenían.
#[tauri::command]
pub fn poner_etiqueta(
    ids: Vec<i64>,
    etiqueta: i64,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<i64>, String> {
    with_library_mut(&estado, |lib| lib.add_tag(&ids, etiqueta))
}

/// Quita una etiqueta a varias fotos. Devuelve las que la tenían.
#[tauri::command]
pub fn quitar_etiqueta(
    ids: Vec<i64>,
    etiqueta: i64,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<i64>, String> {
    with_library_mut(&estado, |lib| lib.remove_tag(&ids, etiqueta))
}

/// Etiquetas de una foto.
#[tauri::command]
pub fn etiquetas_de_foto(id: i64, estado: State<'_, OpenLibrary>) -> Result<Vec<Tag>, String> {
    with_library(&estado, |lib| lib.photo_tags(id))
}

/// Cuántas de las fotos `ids` tiene cada etiqueta.
#[tauri::command]
pub fn uso_etiquetas(
    ids: Vec<i64>,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<TagUsage>, String> {
    with_library(&estado, |lib| lib.tag_usage(&ids))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_renombrado_viaja_como_objeto() {
        let tag = Tag {
            id: 3,
            name: "Playa".into(),
            photos: 2,
            cover: Some(9),
            cover_edit_version: None,
        };
        let json = serde_json::to_value(Renombrado::Existe(tag)).unwrap();
        assert_eq!(json["existe"]["name"], "Playa");
        assert_eq!(json["existe"]["cover"], 9);
    }
}
