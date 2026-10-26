mod media;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use mnemos_core::thumbnails::DEFAULT_WORKERS;
use mnemos_core::{
    Library, LibraryInfo, Month, PhotoDetails, ThumbnailQueue, TimelineCursor, TimelinePage,
};
use tauri::http::{Method, StatusCode, header};
use tauri::{AppHandle, Emitter, Manager, State};

/// La biblioteca abierta. El protocolo `mnemos://` la consulta en cada petición.
#[derive(Default)]
struct OpenLibrary(Mutex<Option<Library>>);

/// La cola de miniaturas de la biblioteca abierta. Se para al cambiar de
/// biblioteca.
#[derive(Default)]
struct Thumbnails(Mutex<Option<Arc<ThumbnailQueue>>>);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

#[tauri::command]
fn abrir_biblioteca(
    ruta: String,
    app: AppHandle,
    estado: State<'_, OpenLibrary>,
    miniaturas: State<'_, Thumbnails>,
) -> Result<LibraryInfo, String> {
    let root = PathBuf::from(ruta);
    let lib = Library::open(&root).map_err(|e| e.to_string())?;
    let info = lib.info().map_err(|e| e.to_string())?;
    // Primero se para la cola anterior, para no mezclar bibliotecas.
    lock(&miniaturas.0).take();
    *lock(&estado.0) = Some(lib);
    start_thumbnails(&app, &root, &miniaturas)?;
    Ok(info)
}

/// Arranca la cola de miniaturas y le pasa en segundo plano las que faltan:
/// buscarlas obliga a mirar el disco foto a foto.
fn start_thumbnails(app: &AppHandle, root: &Path, miniaturas: &Thumbnails) -> Result<(), String> {
    let emitter = app.clone();
    let queue = ThumbnailQueue::start(root, DEFAULT_WORKERS, move |event| {
        let _ = emitter.emit("miniaturas", event);
    })
    .map(Arc::new)
    .map_err(|e| e.to_string())?;
    *lock(&miniaturas.0) = Some(Arc::clone(&queue));
    let root = root.to_owned();
    std::thread::spawn(move || {
        match Library::open(&root).and_then(|lib| lib.pending_thumbnails()) {
            Ok(jobs) => queue.push(jobs),
            Err(e) => eprintln!("no se pudieron buscar las miniaturas que faltan: {e}"),
        }
        // Si entretanto se abrió otra biblioteca, esta es la última referencia
        // y la cola se para aquí.
    });
    Ok(())
}

/// Adelanta las miniaturas de las fotos que se ven en pantalla.
#[tauri::command]
fn priorizar_miniaturas(ids: Vec<i64>, miniaturas: State<'_, Thumbnails>) {
    if let Some(queue) = lock(&miniaturas.0).as_ref() {
        queue.prioritize(&ids);
    }
}

/// Ejecuta `f` con la biblioteca abierta, o falla si no hay ninguna.
fn with_library<T>(
    estado: &OpenLibrary,
    f: impl FnOnce(&Library) -> mnemos_core::Result<T>,
) -> Result<T, String> {
    let guard = lock(&estado.0);
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

/// Detalles de una foto para el panel del visor.
#[tauri::command]
fn detalles(id: i64, estado: State<'_, OpenLibrary>) -> Result<Option<PhotoDetails>, String> {
    with_library(&estado, |lib| lib.photo_details(id))
}

/// Atiende `mnemos://localhost/miniatura/<id>` y `mnemos://localhost/foto/<id>`.
fn serve(
    app: &AppHandle,
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
        let guard = lock(&state.0);
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
        .manage(Thumbnails::default())
        .register_asynchronous_uri_scheme_protocol("mnemos", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            // Decodificar un RAW o un TIFF lleva tiempo: fuera del hilo de la interfaz.
            std::thread::spawn(move || responder.respond(serve(&app, &request)));
        })
        .invoke_handler(tauri::generate_handler![
            abrir_biblioteca,
            meses,
            fotos,
            detalles,
            priorizar_miniaturas
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar MNEMOS");
}
