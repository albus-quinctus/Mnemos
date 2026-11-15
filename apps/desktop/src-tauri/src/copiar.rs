//! Copiar, descargar y «Guardar como…» (spec 020).
//!
//! Copiar pone en el portapapeles del sistema la foto tal como se ve, a
//! tamaño completo. Descargar es exportar en tamaño original a la carpeta de
//! Descargas, sin preguntar. «Guardar como…» guarda en el archivo que el
//! usuario elige en el diálogo del sistema. Nada de esto toca la biblioteca.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use mnemos_core::{Error, ExportSize, PhotoMedia};
use mnemos_imaging::ThumbnailError;
use tauri::image::Image;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::exportacion::{Exportacion, exportar_varias};
use crate::{OpenLibrary, with_library};

/// Raíz de la biblioteca y lo que hace falta para leer la foto `id`.
fn foto(id: i64, estado: &OpenLibrary) -> Result<(PathBuf, PhotoMedia), String> {
    with_library(estado, |lib| {
        let media = lib.photo_media(id)?.ok_or(Error::PhotoNotFound(id))?;
        Ok((lib.root().to_path_buf(), media))
    })
}

/// Cada copia pedida lleva un número; solo la última llega al portapapeles,
/// aunque una anterior tarde más en decodificarse.
static ULTIMA_COPIA: AtomicU64 = AtomicU64::new(0);

/// Copia la foto `id` al portapapeles, tal como se ve. Devuelve `false` si
/// mientras tanto se pidió otra copia, que es la que queda.
#[tauri::command]
pub async fn copiar_imagen(
    id: i64,
    app: AppHandle,
    estado: State<'_, OpenLibrary>,
) -> Result<bool, String> {
    let turno = ULTIMA_COPIA.fetch_add(1, Ordering::SeqCst) + 1;
    let (root, media) = foto(id, &estado)?;
    let src = media.rel_path.to_path(&root);
    if mnemos_imaging::is_video(&src) {
        return Err("Los vídeos no se pueden copiar".into());
    }
    if !src.is_file() {
        return Err("El archivo no está en el disco".into());
    }
    let geometria = media.edit.map(|e| e.geometry).unwrap_or_default();
    let img = tauri::async_runtime::spawn_blocking(move || {
        mnemos_imaging::clipboard_image(&src, media.orientation, &geometria)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(motivo_copia)?;
    if ULTIMA_COPIA.load(Ordering::SeqCst) != turno {
        return Ok(false);
    }
    let (ancho, alto) = img.dimensions();
    app.clipboard()
        .write_image(&Image::new_owned(img.into_raw(), ancho, alto))
        .map_err(|e| format!("No se pudo copiar al portapapeles: {e}"))?;
    Ok(true)
}

/// El motivo de una copia fallida, en lenguaje claro.
fn motivo_copia(err: ThumbnailError) -> String {
    match err {
        ThumbnailError::Unsupported => {
            "Esta foto no se puede copiar todavía. Usa Descargar".to_owned()
        }
        ThumbnailError::Io(e) => format!("No se pudo leer la foto: {e}"),
        ThumbnailError::Decode(e) => format!("No se pudo leer la foto: {e}"),
    }
}

/// La carpeta de Descargas del sistema, si existe.
fn descargas(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .download_dir()
        .ok()
        .filter(|d| d.is_dir())
        .ok_or_else(|| "No se encuentra la carpeta Descargas. Usa Guardar como…".to_owned())
}

/// El error de exportar a Descargas, con el consejo que corresponde.
fn motivo_descarga(err: Error) -> String {
    match err {
        Error::DestinationInsideLibrary(_) => {
            "Tu carpeta Descargas está dentro de la biblioteca. Usa Guardar como…".to_owned()
        }
        other => mayuscula(&other.to_string()),
    }
}

fn mayuscula(texto: &str) -> String {
    let mut letras = texto.chars();
    match letras.next() {
        Some(primera) => primera.to_uppercase().chain(letras).collect(),
        None => String::new(),
    }
}

/// Guarda la foto `id` en Descargas, como «Exportar · Tamaño original», y
/// devuelve la ruta del archivo creado.
#[tauri::command]
pub async fn descargar(
    id: i64,
    app: AppHandle,
    estado: State<'_, OpenLibrary>,
) -> Result<String, String> {
    let carpeta = descargas(&app)?;
    let (root, media) = foto(id, &estado)?;
    tauri::async_runtime::spawn_blocking(move || {
        mnemos_core::export::export(&root, &media, &carpeta, ExportSize::Original)
    })
    .await
    .map_err(|e| e.to_string())?
    .map(|ruta| ruta.display().to_string())
    .map_err(motivo_descarga)
}

/// Descarga varias fotos con el lote de la spec 017 (evento `exportacion`).
/// Devuelve la carpeta de Descargas.
#[tauri::command]
pub fn descargar_varias(
    ids: Vec<i64>,
    app: AppHandle,
    estado: State<'_, OpenLibrary>,
    exportacion: State<'_, Exportacion>,
) -> Result<String, String> {
    let carpeta = descargas(&app)?;
    let raiz = with_library(&estado, |lib| Ok(lib.root().to_path_buf()))?;
    if mnemos_core::export::inside(&raiz, &carpeta).unwrap_or(false) {
        return Err(motivo_descarga(Error::DestinationInsideLibrary(carpeta)));
    }
    let texto = carpeta.display().to_string();
    exportar_varias(
        ids,
        texto.clone(),
        ExportSize::Original,
        app,
        estado,
        exportacion,
    )?;
    Ok(texto)
}

/// «Guardar como…»: guarda la foto `id` en `ruta`, elegida en el diálogo del
/// sistema, y devuelve la ruta final (con la extensión añadida si faltaba).
#[tauri::command]
pub async fn guardar_como(
    id: i64,
    ruta: String,
    estado: State<'_, OpenLibrary>,
) -> Result<String, String> {
    let (root, media) = foto(id, &estado)?;
    tauri::async_runtime::spawn_blocking(move || {
        mnemos_core::export::export_to_file(&root, &media, Path::new(&ruta))
    })
    .await
    .map_err(|e| e.to_string())?
    .map(|ruta| ruta.display().to_string())
    .map_err(|e| mayuscula(&e.to_string()))
}

/// Abre el gestor de archivos en la carpeta de `ruta`, con el archivo
/// marcado donde el sistema lo permite. Si `ruta` es una carpeta (el destino
/// de un lote), abre esa carpeta.
#[tauri::command]
pub fn mostrar_en_carpeta(ruta: String) -> Result<(), String> {
    let ruta = PathBuf::from(ruta);
    if !ruta.exists() {
        return Err("El archivo ya no está ahí".into());
    }
    let mut orden = if ruta.is_dir() {
        orden_abrir_carpeta(&ruta)
    } else {
        orden_mostrar(&ruta)
    };
    orden
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("No se puede abrir la carpeta: {e}"))
}

/// La orden de cada sistema para enseñar un archivo en su carpeta.
fn orden_mostrar(ruta: &Path) -> Command {
    if cfg!(target_os = "windows") {
        // `explorer` quiere `/select,` y la ruta en el mismo argumento.
        let mut arg = std::ffi::OsString::from("/select,");
        arg.push(ruta);
        let mut orden = Command::new("explorer");
        orden.arg(arg);
        orden
    } else if cfg!(target_os = "macos") {
        let mut orden = Command::new("open");
        orden.arg("-R").arg(ruta);
        orden
    } else {
        // `xdg-open` no sabe marcar un archivo: abre su carpeta.
        let carpeta = ruta.parent().unwrap_or(ruta);
        let mut orden = Command::new("xdg-open");
        orden.arg(carpeta);
        orden
    }
}

/// La orden de cada sistema para abrir una carpeta en su gestor de archivos.
fn orden_abrir_carpeta(carpeta: &Path) -> Command {
    let programa = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut orden = Command::new(programa);
    orden.arg(carpeta);
    orden
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_carpeta_va_como_un_solo_argumento() {
        let ruta = Path::new("/home/lilith/Descargas/IMG con espacios & ñ.jpg");
        let orden = orden_mostrar(ruta);
        let args: Vec<_> = orden.get_args().collect();
        if cfg!(target_os = "linux") {
            assert_eq!(args, [Path::new("/home/lilith/Descargas").as_os_str()]);
        } else {
            assert!(args.last().is_some());
        }
    }

    #[test]
    fn los_motivos_se_dicen_claros() {
        assert_eq!(
            motivo_copia(ThumbnailError::Unsupported),
            "Esta foto no se puede copiar todavía. Usa Descargar"
        );
        assert!(
            motivo_descarga(Error::DestinationInsideLibrary(PathBuf::from("/x")))
                .starts_with("Tu carpeta Descargas")
        );
        assert_eq!(
            motivo_descarga(Error::OriginalMissing("a.jpg".into())),
            "El archivo a.jpg no está en el disco"
        );
    }
}
