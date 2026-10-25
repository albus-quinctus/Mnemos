use std::path::PathBuf;

use mnemos_core::{Library, LibraryInfo};

/// Abre la biblioteca en `ruta` y devuelve su resumen.
#[tauri::command]
fn abrir_biblioteca(ruta: String) -> Result<LibraryInfo, String> {
    Library::open(&PathBuf::from(ruta))
        .and_then(|lib| lib.info())
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![abrir_biblioteca])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar MNEMOS");
}
