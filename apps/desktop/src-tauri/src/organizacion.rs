//! Comandos de la spec 006: vistas Fotos, Favoritos y Archivo, selección
//! múltiple, favoritos, archivo y títulos de día. Solo cambian la base de
//! datos; ninguno toca los archivos.

use mnemos_core::{
    DayTitle, Library, Month, Scope, SearchQuery, TimelineCursor, TimelinePage, View,
};
use serde::Deserialize;
use tauri::State;

use crate::{OpenLibrary, with_library, with_library_mut};

/// Dónde se selecciona: `{"vista": "favoritos"}` o `{"busqueda": "playa"}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ambito {
    Vista(View),
    Busqueda(String),
}

impl Ambito {
    /// Llama a `f` con el `Scope` del núcleo; la consulta de la búsqueda
    /// tiene que vivir mientras dure la llamada.
    fn con_scope<T>(&self, f: impl FnOnce(Scope<'_>) -> T) -> T {
        match self {
            Ambito::Vista(vista) => f(Scope::View(*vista)),
            Ambito::Busqueda(texto) => f(Scope::Search(&SearchQuery::parse(texto))),
        }
    }
}

/// Meses con fotos de una vista (Fotos sin archivadas, Favoritos o Archivo).
#[tauri::command]
pub fn vista_meses(vista: View, estado: State<'_, OpenLibrary>) -> Result<Vec<Month>, String> {
    with_library(&estado, |lib| lib.view_months(vista))
}

/// Página de fotos de una vista, con el mismo cursor que `fotos`.
#[tauri::command]
pub fn vista_fotos(
    vista: View,
    despues: Option<TimelineCursor>,
    limite: u32,
    estado: State<'_, OpenLibrary>,
) -> Result<TimelinePage, String> {
    with_library(&estado, |lib| lib.view_page(vista, despues, limite))
}

/// Añade a Favoritos (`favorito = true`) o quita. Devuelve los ids que han
/// cambiado, para «Deshacer».
#[tauri::command]
pub fn marcar_favoritos(
    ids: Vec<i64>,
    favorito: bool,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<i64>, String> {
    with_library_mut(&estado, |lib| lib.set_favorite(&ids, favorito))
}

/// Archiva (`archivar = true`) o desarchiva. Devuelve los ids que han
/// cambiado, para «Deshacer».
#[tauri::command]
pub fn archivar(
    ids: Vec<i64>,
    archivar: bool,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<i64>, String> {
    with_library_mut(&estado, |lib| lib.set_archived(&ids, archivar))
}

/// Ids de todo el ámbito, en el orden de la cuadrícula (`Ctrl` + `A`).
#[tauri::command]
pub fn seleccionar_todo(
    ambito: Ambito,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<i64>, String> {
    with_library(&estado, |lib| ambito.con_scope(|s| lib.select_all(s)))
}

/// Ids de un día (`"2024-10-12"`, hora local) dentro del ámbito.
#[tauri::command]
pub fn seleccionar_dia(
    ambito: Ambito,
    dia: String,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<i64>, String> {
    with_library(&estado, |lib| ambito.con_scope(|s| lib.select_day(s, &dia)))
}

/// Ids entre dos fotos del ámbito, ambas incluidas (`Mayús` + clic).
#[tauri::command]
pub fn seleccionar_rango(
    ambito: Ambito,
    desde: TimelineCursor,
    hasta: TimelineCursor,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<i64>, String> {
    with_library(&estado, |lib| {
        ambito.con_scope(|s| lib.select_range(s, desde, hasta))
    })
}

/// Todos los títulos de día de la biblioteca.
#[tauri::command]
pub fn titulos_dia(estado: State<'_, OpenLibrary>) -> Result<Vec<DayTitle>, String> {
    with_library(&estado, Library::day_titles)
}

/// Guarda el título de un día y devuelve cómo ha quedado (`null` si se ha
/// borrado).
#[tauri::command]
pub fn poner_titulo_dia(
    dia: String,
    titulo: String,
    estado: State<'_, OpenLibrary>,
) -> Result<Option<String>, String> {
    with_library(&estado, |lib| lib.set_day_title(&dia, &titulo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_ambito_se_lee_de_json() {
        let vista: Ambito = serde_json::from_str(r#"{"vista": "archivo"}"#).unwrap();
        assert!(matches!(vista, Ambito::Vista(View::Archive)));
        let busqueda: Ambito = serde_json::from_str(r#"{"busqueda": "playa 2024"}"#).unwrap();
        assert!(matches!(busqueda, Ambito::Busqueda(ref t) if t == "playa 2024"));
        assert!(serde_json::from_str::<Ambito>(r#"{"vista": "papelera"}"#).is_err());
    }
}
