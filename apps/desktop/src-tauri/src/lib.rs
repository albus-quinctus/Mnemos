mod media;

use std::path::PathBuf;
use std::sync::Mutex;

use mnemos_core::{Library, LibraryInfo, Month, TimelineCursor, TimelinePage};
use tauri::http::{Method, StatusCode, header};
use tauri::{Manager, State};

/// La biblioteca abierta. El protocolo `mnemos://` la consulta en cada petición.
#[derive(Default)]
struct OpenLibrary(Mutex<Option<Library>>);

#[tauri::command]
fn abrir_biblioteca(ruta: String, estado: State<'_, OpenLibrary>) -> Result<LibraryInfo, String> {
    let lib = Library::open(&PathBuf::from(ruta)).map_err(|e| e.to_string())?;
    let info = lib.info().map_err(|e| e.to_string())?;
    *estado.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(lib);
    Ok(info)
}

/// Ejecuta `f` con la biblioteca abierta, o falla si no hay ninguna.
fn with_library<T>(
    estado: &OpenLibrary,
    f: impl FnOnce(&Library) -> mnemos_core::Result<T>,
) -> Result<T, String> {
    let guard = estado.0.lock().unwrap_or_else(|e| e.into_inner());
    let lib = guard.as_ref().ok_or("no hay ninguna biblioteca abierta")?;
    f(lib).map_err(|e| e.to_string())
}

/// Meses con fotos y cuántas tiene cada uno, del más reciente al más antiguo.
#[tauri::command]
fn meses(estado: State<'_, OpenLibrary>) -> Result<Vec<Month>, String> {
    with_library(&estado, Library::timeline_months)
}

/// Página de fotos para la cuadrícula, a partir de `despues` o desde la más
/// reciente.
#[tauri::command]
fn fotos(
    despues: Option<TimelineCursor>,
    limite: u32,
    estado: State<'_, OpenLibrary>,
) -> Result<TimelinePage, String> {
    with_library(&estado, |lib| lib.timeline_page(despues, limite))
}

/// Atiende `mnemos://localhost/miniatura/<id>` y `mnemos://localhost/foto/<id>`.
fn serve(
    app: &tauri::AppHandle,
    request: &tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    if request.method() != Method::GET {
        return media::status(StatusCode::METHOD_NOT_ALLOWED);
    }
    let Some((kind, id)) = media::parse(request.uri().path()) else {
        return media::status(StatusCode::NOT_FOUND);
    };
    // Solo se bloquea la biblioteca para traducir el id; los archivos se leen
    // fuera del bloqueo.
    let found = {
        let state = app.state::<OpenLibrary>();
        let guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .as_ref()
            .map(|lib| (lib.root().to_path_buf(), lib.photo_media(id)))
    };
    let (root, media) = match found {
        Some((root, Ok(Some(media)))) => (root, media),
        Some((_, Err(_))) => return media::status(StatusCode::INTERNAL_SERVER_ERROR),
        _ => return media::status(StatusCode::NOT_FOUND),
    };
    let if_none_match = request
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok());
    media::respond(&root, kind, &media, if_none_match)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(OpenLibrary::default())
        .register_asynchronous_uri_scheme_protocol("mnemos", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            // Decodificar un RAW o un TIFF lleva tiempo: fuera del hilo de la interfaz.
            std::thread::spawn(move || responder.respond(serve(&app, &request)));
        })
        .invoke_handler(tauri::generate_handler![abrir_biblioteca, meses, fotos])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar MNEMOS");
}
