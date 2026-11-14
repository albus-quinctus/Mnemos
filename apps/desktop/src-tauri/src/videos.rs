//! Comandos de los vídeos (spec 019): datos del contenedor para Detalles y
//! abrir con el programa del sistema lo que el motor web no reproduce.

use std::path::{Path, PathBuf};
use std::process::Command;

use mnemos_imaging::VideoInfo;
use tauri::State;

use crate::{OpenLibrary, with_library};

/// Ruta del vídeo `id`, o `None` si no existe o no es un vídeo.
fn ruta_video(id: i64, estado: &OpenLibrary) -> Result<Option<PathBuf>, String> {
    with_library(estado, |lib| {
        Ok(lib
            .photo_media(id)?
            .map(|media| media.rel_path.to_path(lib.root()))
            .filter(|ruta| mnemos_imaging::is_video(ruta)))
    })
}

/// Duración, fotogramas, códecs y demás que dice el contenedor del vídeo. Es
/// asíncrono para leer el disco fuera del hilo de la interfaz.
#[tauri::command]
pub async fn datos_video(
    id: i64,
    estado: State<'_, OpenLibrary>,
) -> Result<Option<VideoInfo>, String> {
    Ok(ruta_video(id, &estado)?.and_then(|ruta| mnemos_imaging::video_info(&ruta)))
}

/// Abre el vídeo con el programa que el sistema tenga para él. Se abre el
/// original en su sitio, sin copiarlo; MNEMOS no lo toca.
#[tauri::command]
pub fn abrir_con_otra_aplicacion(id: i64, estado: State<'_, OpenLibrary>) -> Result<(), String> {
    let ruta = ruta_video(id, &estado)?.ok_or("no es un vídeo de la biblioteca")?;
    if !ruta.is_file() {
        return Err("el archivo no está en el disco".into());
    }
    orden_abrir(&ruta)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("no se puede abrir el vídeo: {e}"))
}

/// La orden de cada sistema para abrir un archivo con su programa.
fn orden_abrir(ruta: &Path) -> Command {
    if cfg!(target_os = "windows") {
        // `start` es de `cmd`; el primer argumento entre comillas es el título.
        let mut orden = Command::new("cmd");
        orden.args(["/C", "start", ""]).arg(ruta);
        orden
    } else if cfg!(target_os = "macos") {
        let mut orden = Command::new("open");
        orden.arg(ruta);
        orden
    } else {
        let mut orden = Command::new("xdg-open");
        orden.arg(ruta);
        orden
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_ruta_va_como_un_solo_argumento() {
        let ruta = Path::new("/media/FOTOS/Originales/2019/05/VID con espacios & ñ.mp4");
        let orden = orden_abrir(ruta);
        let args: Vec<_> = orden.get_args().collect();
        assert_eq!(args.last().copied(), Some(ruta.as_os_str()));
    }
}
