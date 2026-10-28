mod configuracion;
mod media;
mod portatil;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use mnemos_core::thumbnails::DEFAULT_WORKERS;
use mnemos_core::{
    ImportReport, Library, LibraryInfo, Month, PhotoDetails, Progreso, SearchQuery, ThumbnailQueue,
    TimelineCursor, TimelinePage,
};
use serde::Serialize;
use tauri::http::{Method, StatusCode, header};
use tauri::{AppHandle, Emitter, Manager, State};

/// La biblioteca abierta. El protocolo `mnemos://` la consulta en cada petición.
#[derive(Default)]
struct OpenLibrary(Mutex<Option<Library>>);

/// La cola de miniaturas de la biblioteca abierta. Se para al cambiar de
/// biblioteca.
#[derive(Default)]
struct Thumbnails(Mutex<Option<Arc<ThumbnailQueue>>>);

/// La importación en marcha, si la hay. Guarda la señal para pararla.
#[derive(Default)]
struct Importacion(Mutex<Option<Arc<AtomicBool>>>);

/// Raíz del disco si la app se ejecuta en modo portátil (spec 005).
struct Portatil(Option<PathBuf>);

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
    let ruta = PathBuf::from(ruta);
    if ruta.join(".mnemos").is_dir() && !portatil::se_puede_escribir(&ruta) {
        return Err(format!(
            "No se puede escribir en {}. El disco está en solo lectura: macOS no \
             escribe en discos NTFS, y Windows o Linux montan así un disco con errores.",
            ruta.display()
        ));
    }
    let lib = Library::open(&ruta).map_err(|e| e.to_string())?;
    adoptar(lib, &app, &estado, &miniaturas)
}

/// Crea una biblioteca en la carpeta elegida y la abre.
#[tauri::command]
fn crear_biblioteca(
    ruta: String,
    app: AppHandle,
    estado: State<'_, OpenLibrary>,
    miniaturas: State<'_, Thumbnails>,
) -> Result<LibraryInfo, String> {
    let lib = Library::create(&PathBuf::from(ruta)).map_err(|e| e.to_string())?;
    adoptar(lib, &app, &estado, &miniaturas)
}

/// Ruta de la última biblioteca abierta, si se recuerda. En modo portátil,
/// si no hay ninguna, la carpeta `MNEMOS/` del disco.
#[tauri::command]
fn ultima_biblioteca(app: AppHandle) -> Result<Option<String>, String> {
    let carpeta = carpeta_config(&app)?;
    let config = configuracion::Configuracion::leer(&carpeta);
    let raiz = app.state::<Portatil>().0.clone();
    let ruta = match (config.ultima_biblioteca, raiz) {
        (Some(ruta), Some(raiz)) => Some(portatil::resolver(&raiz, &ruta)),
        (None, Some(raiz)) => portatil::biblioteca_por_defecto(&raiz),
        (ruta, None) => ruta,
    };
    Ok(ruta.map(|ruta| ruta.display().to_string()))
}

/// Deja `lib` como biblioteca activa, arranca sus miniaturas y la recuerda.
fn adoptar(
    lib: Library,
    app: &AppHandle,
    estado: &OpenLibrary,
    miniaturas: &Thumbnails,
) -> Result<LibraryInfo, String> {
    let info = lib.info().map_err(|e| e.to_string())?;
    let root = lib.root().to_path_buf();
    // Primero se para la cola anterior, para no mezclar bibliotecas.
    lock(&miniaturas.0).take();
    *lock(&estado.0) = Some(lib);
    start_thumbnails(app, &root, miniaturas)?;
    recordar(app, &root);
    Ok(info)
}

fn carpeta_config(app: &AppHandle) -> Result<PathBuf, String> {
    match &app.state::<Portatil>().0 {
        Some(raiz) => Ok(portatil::carpeta_config(raiz)),
        None => app.path().app_config_dir().map_err(|e| e.to_string()),
    }
}

/// Guarda la biblioteca como la última abierta. Si falla, la biblioteca sigue
/// abierta: solo se pierde el recuerdo para el próximo arranque.
fn recordar(app: &AppHandle, root: &Path) {
    let ruta = match &app.state::<Portatil>().0 {
        Some(raiz) => portatil::para_guardar(raiz, root),
        None => root.to_owned(),
    };
    let config = configuracion::Configuracion {
        ultima_biblioteca: Some(ruta),
    };
    let guardado = carpeta_config(app).and_then(|carpeta| {
        config
            .guardar(&carpeta)
            .map_err(|e| format!("no se pudo guardar la configuración: {e}"))
    });
    if let Err(e) = guardado {
        eprintln!("{e}");
    }
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

/// Meses con resultados para `texto` y cuántos tiene cada uno (spec 003).
#[tauri::command]
fn buscar_meses(texto: String, estado: State<'_, OpenLibrary>) -> Result<Vec<Month>, String> {
    let consulta = SearchQuery::parse(&texto);
    with_library(&estado, |lib| lib.search_months(&consulta))
}

/// Página de resultados para `texto`, con el mismo cursor que `fotos`.
#[tauri::command]
fn buscar_fotos(
    texto: String,
    despues: Option<TimelineCursor>,
    limite: u32,
    estado: State<'_, OpenLibrary>,
) -> Result<TimelinePage, String> {
    let consulta = SearchQuery::parse(&texto);
    with_library(&estado, |lib| lib.search_page(&consulta, despues, limite))
}

/// Detalles de una foto para el panel del visor.
#[tauri::command]
fn detalles(id: i64, estado: State<'_, OpenLibrary>) -> Result<Option<PhotoDetails>, String> {
    with_library(&estado, |lib| lib.photo_details(id))
}

/// Lo que la interfaz recibe sobre una importación. `estado` puede ser
/// `en_curso`, `terminada`, `cancelada` o `fallida`.
#[derive(Serialize, Clone, Default)]
struct EventoImportacion {
    estado: &'static str,
    total: u64,
    procesadas: u64,
    copiadas: u64,
    duplicadas: u64,
    errores: usize,
    mensaje: Option<String>,
}

impl EventoImportacion {
    fn en_curso(progreso: Progreso, parcial: &ImportReport) -> Self {
        Self {
            estado: "en_curso",
            total: progreso.total,
            procesadas: progreso.procesadas,
            copiadas: parcial.copied,
            duplicadas: parcial.duplicates,
            errores: parcial.errors.len(),
            mensaje: None,
        }
    }

    fn final_de(informe: &ImportReport) -> Self {
        Self {
            estado: if informe.cancelled {
                "cancelada"
            } else {
                "terminada"
            },
            copiadas: informe.copied,
            duplicadas: informe.duplicates,
            errores: informe.errors.len(),
            ..Self::default()
        }
    }

    fn fallida(mensaje: String) -> Self {
        Self {
            estado: "fallida",
            mensaje: Some(mensaje),
            ..Self::default()
        }
    }
}

/// Importa `origen` a la biblioteca de `root` desde una conexión propia, para
/// no bloquear la biblioteca abierta mientras se copia.
fn ejecutar_importacion(
    app: &AppHandle,
    root: &Path,
    origen: &Path,
    cancelar: &AtomicBool,
) -> Result<ImportReport, String> {
    let mut lib = Library::open(root).map_err(|e| e.to_string())?;
    let informe = lib
        .import_from_with(origen, cancelar, |progreso, parcial| {
            let _ = app.emit(
                "importacion",
                EventoImportacion::en_curso(progreso, parcial),
            );
        })
        .map_err(|e| e.to_string())?;
    // Las fotos nuevas aún no tienen miniatura: se encolan.
    let queue = lock(&app.state::<Thumbnails>().0).clone();
    if let Some(queue) = queue {
        match lib.pending_thumbnails() {
            Ok(jobs) => queue.push(jobs),
            Err(e) => eprintln!("no se pudieron encolar las miniaturas nuevas: {e}"),
        }
    }
    Ok(informe)
}

/// Inicia la importación de `origen` en segundo plano. Los avances llegan como
/// eventos `importacion`; solo puede haber una a la vez.
#[tauri::command]
fn importar(
    origen: String,
    app: AppHandle,
    estado: State<'_, OpenLibrary>,
    importacion: State<'_, Importacion>,
) -> Result<(), String> {
    let root = with_library(&estado, |lib| Ok(lib.root().to_path_buf()))?;
    let cancelar = Arc::new(AtomicBool::new(false));
    {
        let mut activa = lock(&importacion.0);
        if activa.is_some() {
            return Err("ya hay una importación en marcha".to_owned());
        }
        *activa = Some(Arc::clone(&cancelar));
    }
    let emisor = app.clone();
    std::thread::spawn(move || {
        let resultado = ejecutar_importacion(&emisor, &root, Path::new(&origen), &cancelar);
        let evento = match resultado {
            Ok(informe) => EventoImportacion::final_de(&informe),
            Err(mensaje) => EventoImportacion::fallida(mensaje),
        };
        let _ = emisor.emit("importacion", evento);
        lock(&emisor.state::<Importacion>().0).take();
    });
    Ok(())
}

/// Pide parar la importación en marcha. Lo ya copiado se conserva.
#[tauri::command]
fn cancelar_importacion(importacion: State<'_, Importacion>) {
    if let Some(cancelar) = lock(&importacion.0).as_ref() {
        cancelar.store(true, Ordering::Relaxed);
    }
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

/// Crea la ventana principal. En modo portátil, los datos del WebView (caché,
/// almacenamiento local) se guardan en el disco y no en el perfil del usuario.
fn crear_ventana(app: &AppHandle, raiz: Option<&Path>) -> tauri::Result<()> {
    let mut ventana = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
        .title("MNEMOS")
        .inner_size(1200.0, 800.0)
        .min_inner_size(640.0, 480.0);
    if let Some(raiz) = raiz {
        ventana = ventana.data_directory(portatil::carpeta_webview(raiz));
    }
    ventana.build()?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let raiz = portatil::raiz();
    tauri::Builder::default()
        .setup(|app| {
            crear_ventana(app.handle(), app.state::<Portatil>().0.as_deref())?;
            Ok(())
        })
        .manage(Portatil(raiz))
        .manage(OpenLibrary::default())
        .manage(Thumbnails::default())
        .manage(Importacion::default())
        .plugin(tauri_plugin_dialog::init())
        .register_asynchronous_uri_scheme_protocol("mnemos", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            // Decodificar un RAW o un TIFF lleva tiempo: fuera del hilo de la interfaz.
            std::thread::spawn(move || responder.respond(serve(&app, &request)));
        })
        .invoke_handler(tauri::generate_handler![
            abrir_biblioteca,
            crear_biblioteca,
            ultima_biblioteca,
            meses,
            fotos,
            buscar_meses,
            buscar_fotos,
            detalles,
            priorizar_miniaturas,
            importar,
            cancelar_importacion
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar MNEMOS");
}
