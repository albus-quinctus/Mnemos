//! Comandos de la vista de duplicados (spec 010). Solo consultan: ninguno
//! borra, mueve ni marca fotos.

use mnemos_core::{DuplicateCopy, DuplicateCursor, DuplicatePage, DuplicateSummary, Library};
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

/// Las otras fotos con la misma huella que `id` (spec 018): comparten su
/// receta, así que el editor avisa antes de guardar como original.
#[tauri::command]
pub fn copias_identicas(
    id: i64,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<DuplicateCopy>, String> {
    with_library(&estado, |lib| lib.identical_copies(id))
}
