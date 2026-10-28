//! Modo portátil (spec 005): la app se ejecuta desde el disco de la biblioteca
//! y guarda allí sus ajustes, sin dejar rastro en el equipo.
//!
//! Se activa cuando hay un [`MARCA`] junto al ejecutable. «Junto» es la carpeta
//! del `.exe`, la del `.AppImage` (no la carpeta temporal donde se monta) o la
//! que contiene `MNEMOS.app`.

use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Archivo que activa el modo portátil.
pub const MARCA: &str = "mnemos-portatil.json";

/// Carpeta de ajustes y datos del WebView en el disco.
pub const CARPETA: &str = ".mnemos-portatil";

/// Carpeta de la biblioteca que se abre si no hay ninguna recordada.
pub const BIBLIOTECA: &str = "MNEMOS";

/// Raíz del disco en modo portátil, o `None` si la app está instalada.
pub fn raiz() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let appimage = std::env::var_os("APPIMAGE");
    raiz_desde(&exe, appimage.as_deref().map(Path::new))
}

/// Como [`raiz`], con el ejecutable y la variable `APPIMAGE` dados.
pub fn raiz_desde(exe: &Path, appimage: Option<&Path>) -> Option<PathBuf> {
    let junto = match appimage {
        Some(appimage) => appimage.parent()?.to_owned(),
        None => carpeta_del_ejecutable(exe)?,
    };
    junto.join(MARCA).is_file().then_some(junto)
}

/// Carpeta que contiene el ejecutable o, en macOS, el paquete `.app`.
fn carpeta_del_ejecutable(exe: &Path) -> Option<PathBuf> {
    let carpeta = exe.parent()?;
    // MNEMOS.app/Contents/MacOS/mnemos-desktop → la carpeta de MNEMOS.app.
    if carpeta.file_name() == Some(OsStr::new("MacOS")) {
        let contents = carpeta.parent()?;
        let app = contents.parent()?;
        if contents.file_name() == Some(OsStr::new("Contents"))
            && app.extension() == Some(OsStr::new("app"))
        {
            return Some(app.parent()?.to_owned());
        }
    }
    Some(carpeta.to_owned())
}

/// Carpeta de ajustes en el disco.
pub fn carpeta_config(raiz: &Path) -> PathBuf {
    raiz.join(CARPETA)
}

/// Carpeta de datos del WebView (caché, almacenamiento local) en el disco.
pub fn carpeta_webview(raiz: &Path) -> PathBuf {
    raiz.join(CARPETA).join("webview")
}

/// Ruta que se guarda para recordar una biblioteca: relativa a la raíz del
/// disco si está dentro, porque la letra o el punto de montaje cambia de un
/// equipo a otro.
pub fn para_guardar(raiz: &Path, biblioteca: &Path) -> PathBuf {
    match biblioteca.strip_prefix(raiz) {
        Ok(relativa) if relativa.as_os_str().is_empty() => PathBuf::from("."),
        Ok(relativa) => relativa.to_owned(),
        Err(_) => biblioteca.to_owned(),
    }
}

/// Ruta absoluta de una biblioteca recordada con [`para_guardar`].
pub fn resolver(raiz: &Path, guardada: &Path) -> PathBuf {
    if guardada.is_absolute() {
        return guardada.to_owned();
    }
    let mut ruta = raiz.to_owned();
    for parte in guardada.components() {
        match parte {
            Component::Normal(p) => ruta.push(p),
            // Nada de `..`: una ruta guardada no puede salir del disco.
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {}
        }
    }
    ruta
}

/// Biblioteca que se abre si no hay ninguna recordada: `MNEMOS/` en la raíz
/// del disco, si lo es.
pub fn biblioteca_por_defecto(raiz: &Path) -> Option<PathBuf> {
    let ruta = raiz.join(BIBLIOTECA);
    ruta.join(".mnemos")
        .join("mnemos.db")
        .is_file()
        .then_some(ruta)
}

/// Indica si se puede escribir en la biblioteca (crear y borrar un archivo en
/// su carpeta `.mnemos/`). macOS no escribe en NTFS y un disco dañado puede
/// montarse en solo lectura.
pub fn se_puede_escribir(biblioteca: &Path) -> bool {
    let prueba = biblioteca
        .join(".mnemos")
        .join(format!(".prueba-escritura-{}", std::process::id()));
    let ok = fs::write(&prueba, b"").is_ok();
    let _ = fs::remove_file(&prueba);
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disco() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(MARCA), "{}").unwrap();
        dir
    }

    #[test]
    fn sin_marca_no_es_portatil() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(raiz_desde(&dir.path().join("MNEMOS.exe"), None), None);
    }

    #[test]
    fn la_marca_junto_al_ejecutable_activa_el_modo_portatil() {
        let dir = disco();
        let raiz = raiz_desde(&dir.path().join("MNEMOS.exe"), None);
        assert_eq!(raiz.as_deref(), Some(dir.path()));
    }

    #[test]
    fn en_macos_cuenta_la_carpeta_del_paquete() {
        let dir = disco();
        let exe = dir.path().join("MNEMOS.app/Contents/MacOS/mnemos-desktop");
        assert_eq!(raiz_desde(&exe, None).as_deref(), Some(dir.path()));
    }

    #[test]
    fn con_appimage_cuenta_su_carpeta_y_no_la_del_montaje() {
        let dir = disco();
        let montaje = Path::new("/tmp/.mount_MNEMOSxyz/usr/bin/mnemos-desktop");
        let appimage = dir.path().join("MNEMOS-linux.AppImage");
        assert_eq!(
            raiz_desde(montaje, Some(&appimage)).as_deref(),
            Some(dir.path())
        );
    }

    #[test]
    fn los_ajustes_y_el_webview_van_al_disco() {
        let raiz = Path::new("/media/u/DISCO");
        assert!(carpeta_config(raiz).starts_with(raiz));
        assert!(carpeta_webview(raiz).starts_with(raiz));
    }

    #[test]
    fn la_biblioteca_se_recuerda_relativa_y_sigue_al_disco() {
        let antes = Path::new("/media/lilith/BACKUP_FOTOS");
        let guardada = para_guardar(antes, &antes.join("MNEMOS"));
        assert_eq!(guardada, PathBuf::from("MNEMOS"));
        // El mismo disco montado en otro sitio, en otro equipo.
        let despues = Path::new("/Volumes/BACKUP_FOTOS");
        assert_eq!(resolver(despues, &guardada), despues.join("MNEMOS"));
        // La raíz misma.
        assert_eq!(resolver(despues, &para_guardar(antes, antes)), despues);
    }

    #[test]
    fn una_biblioteca_fuera_del_disco_se_recuerda_entera() {
        let raiz = Path::new("/media/u/DISCO");
        let fuera = Path::new("/home/u/Fotos");
        assert_eq!(resolver(raiz, &para_guardar(raiz, fuera)), fuera);
    }

    #[test]
    fn una_ruta_guardada_no_sale_del_disco() {
        let raiz = Path::new("/media/u/DISCO");
        assert_eq!(resolver(raiz, Path::new("../../etc")), raiz.join("etc"));
    }

    #[test]
    fn abre_mnemos_si_es_una_biblioteca() {
        let dir = disco();
        assert_eq!(biblioteca_por_defecto(dir.path()), None);
        let lib = dir.path().join(BIBLIOTECA);
        mnemos_core::Library::create(&lib).unwrap();
        assert_eq!(biblioteca_por_defecto(dir.path()), Some(lib));
    }

    #[test]
    fn detecta_si_se_puede_escribir() {
        let dir = disco();
        let lib = dir.path().join(BIBLIOTECA);
        mnemos_core::Library::create(&lib).unwrap();
        assert!(se_puede_escribir(&lib));
        assert!(!se_puede_escribir(&dir.path().join("no-existe")));
        assert_eq!(
            fs::read_dir(lib.join(".mnemos"))
                .unwrap()
                .filter(|e| {
                    e.as_ref()
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".prueba")
                })
                .count(),
            0
        );
    }
}
