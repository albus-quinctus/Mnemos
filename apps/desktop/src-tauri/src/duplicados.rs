//! Comandos de la vista de duplicados (spec 010). Solo consultan: ninguno
//! borra, mueve ni marca fotos.

use mnemos_core::{DuplicateCursor, DuplicatePage, DuplicateSummary, Library};
use tauri::State;

use crate::{OpenLibrary, with_library};

/// Grupos, copias de más y espacio que ocupan.
#[tauri::command]
pub fn duplicados_resumen(estado: State<'_, OpenLibrary>) -> Result<DuplicateSummary, String> {
    with_library(&estado, Library::duplicate_summary)
}

/// Página de grupos de fotos repetidas, a partir de `despues` o desde el más
/// reciente.
#[tauri::command]
pub fn duplicados_grupos(
    despues: Option<DuplicateCursor>,
    limite: u32,
    estado: State<'_, OpenLibrary>,
) -> Result<DuplicatePage, String> {
    with_library(&estado, |lib| {
        lib.duplicate_groups(despues.as_ref(), limite)
    })
}
