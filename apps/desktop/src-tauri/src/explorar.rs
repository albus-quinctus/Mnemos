//! Comando de la spec 021: la pantalla Explorar. Solo lee la base de datos;
//! no toca ningún archivo.

use std::time::{SystemTime, UNIX_EPOCH};

use mnemos_core::Explore;
use tauri::State;

use crate::{OpenLibrary, with_library};

/// Las tres filas de Explorar (recuerdos, colecciones y etiquetas) para el
/// momento actual del equipo.
#[tauri::command]
pub fn explorar(estado: State<'_, OpenLibrary>) -> Result<Explore, String> {
    let hoy = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    with_library(&estado, |lib| lib.explore(hoy))
}

#[cfg(test)]
mod tests {
    use mnemos_core::Library;

    /// Los nombres de los campos son los que espera `Explorar` en
    /// `biblioteca.ts`.
    #[test]
    fn el_json_tiene_los_campos_de_la_interfaz() {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        lib.connection()
            .execute(
                "INSERT INTO photos (rel_path, size_bytes, modified_at, taken_at, imported_at,
                                     media_kind, duration_ms)
                 VALUES ('Originales/v.mp4', 1, 0, 1700000000, 5, 'video', 1000)",
                [],
            )
            .unwrap();
        let hoy = 1_700_000_000 + 365 * 86_400;
        let json = serde_json::to_value(lib.explore(hoy).unwrap()).unwrap();
        let claves = |v: &serde_json::Value| {
            let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
            k.sort();
            k
        };
        assert_eq!(
            claves(&json),
            ["collections", "memories", "tags", "tags_total"]
        );
        assert_eq!(
            claves(&json["memories"][0]),
            [
                "cover",
                "cover_edit_version",
                "desde",
                "hasta",
                "photos",
                "years_ago"
            ]
        );
        assert_eq!(
            claves(&json["collections"][0]),
            [
                "collection",
                "cover",
                "cover_duration_ms",
                "cover_edit_version",
                "last_import",
                "photos"
            ]
        );
        assert_eq!(json["collections"][0]["collection"], "videos");
        assert_eq!(json["collections"][1]["collection"], "recientes");
    }
}
