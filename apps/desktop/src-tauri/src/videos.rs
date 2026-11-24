//! Comandos de los vídeos (spec 019): datos del contenedor para Detalles y
//! abrir con el programa del sistema lo que el motor web no reproduce.

use std::ffi::OsString;
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

/// Variables que el `AppRun` de la AppImage apunta a sus propias librerías y
/// datos. Si las hereda el programa externo, carga esas librerías en vez de las
/// del sistema y puede morir al arrancar (por ejemplo `mpv` con su `libva`).
const VARIABLES_DE_LA_APPIMAGE: &[&str] = &[
    "LD_LIBRARY_PATH",
    "GTK_DATA_PREFIX",
    "GTK_EXE_PREFIX",
    "GTK_PATH",
    "GTK_IM_MODULE_FILE",
    "GDK_PIXBUF_MODULE_FILE",
    "GIO_MODULE_DIR",
    "GI_TYPELIB_PATH",
    "GSETTINGS_SCHEMA_DIR",
    "GST_REGISTRY_REUSE_PLUGIN_SCANNER",
    "GST_PLUGIN_SYSTEM_PATH_1_0",
    "GST_PLUGIN_PATH_1_0",
    "GST_PLUGIN_SCANNER_1_0",
    "GST_PTP_HELPER_1_0",
    "PYTHONHOME",
    "PYTHONPATH",
    "PERLLIB",
    "QT_PLUGIN_PATH",
];

/// Listas de rutas a las que la AppImage añade las suyas al principio.
const LISTAS_DE_RUTAS: &[&str] = &["PATH", "XDG_DATA_DIRS"];

/// Si MNEMOS corre desde una AppImage (`APPDIR`), deja al programa externo sin
/// el entorno de la AppImage: quita sus variables y, de las listas de rutas,
/// solo las entradas que cuelgan de ella. Fuera de una AppImage no toca nada.
fn sin_entorno_de_la_appimage(orden: &mut Command, var: impl Fn(&str) -> Option<OsString>) {
    let Some(appdir) = var("APPDIR").filter(|v| !v.is_empty()) else {
        return;
    };
    for nombre in VARIABLES_DE_LA_APPIMAGE {
        orden.env_remove(nombre);
    }
    for nombre in LISTAS_DE_RUTAS {
        let Some(valor) = var(nombre) else { continue };
        let limpia = std::env::split_paths(&valor)
            .filter(|ruta| !ruta.starts_with(Path::new(&appdir)))
            .collect::<Vec<_>>();
        match std::env::join_paths(limpia) {
            Ok(nueva) => orden.env(nombre, nueva),
            Err(_) => orden.env_remove(nombre),
        };
    }
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
        sin_entorno_de_la_appimage(&mut orden, |n| std::env::var_os(n));
        orden
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    fn entorno_de_prueba(nombre: &str) -> Option<OsString> {
        match nombre {
            "APPDIR" => Some("/tmp/.mount_MNEMOSab".into()),
            "PATH" => Some("/tmp/.mount_MNEMOSab/usr/bin:/usr/bin:/bin".into()),
            "XDG_DATA_DIRS" => Some("/tmp/.mount_MNEMOSab/usr/share:/usr/share".into()),
            _ => None,
        }
    }

    fn valor_fijado<'a>(orden: &'a Command, nombre: &str) -> Option<Option<&'a OsStr>> {
        orden
            .get_envs()
            .find(|(n, _)| *n == OsStr::new(nombre))
            .map(|(_, v)| v)
    }

    #[test]
    fn desde_la_appimage_el_programa_externo_no_hereda_sus_librerias() {
        let mut orden = Command::new("xdg-open");
        sin_entorno_de_la_appimage(&mut orden, entorno_de_prueba);
        for nombre in VARIABLES_DE_LA_APPIMAGE {
            assert_eq!(valor_fijado(&orden, nombre), Some(None), "{nombre}");
        }
        assert_eq!(
            valor_fijado(&orden, "PATH"),
            Some(Some(OsStr::new("/usr/bin:/bin")))
        );
        assert_eq!(
            valor_fijado(&orden, "XDG_DATA_DIRS"),
            Some(Some(OsStr::new("/usr/share")))
        );
    }

    #[test]
    fn fuera_de_una_appimage_no_se_toca_el_entorno() {
        let mut orden = Command::new("xdg-open");
        sin_entorno_de_la_appimage(&mut orden, |_| None);
        assert_eq!(orden.get_envs().count(), 0);
    }

    #[test]
    fn la_ruta_va_como_un_solo_argumento() {
        let ruta = Path::new("/media/FOTOS/Originales/2019/05/VID con espacios & ñ.mp4");
        let orden = orden_abrir(ruta);
        let args: Vec<_> = orden.get_args().collect();
        assert_eq!(args.last().copied(), Some(ruta.as_os_str()));
    }
}
