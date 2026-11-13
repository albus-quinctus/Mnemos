//! Arranque de la AppImage en compositores Wayland sin escritorio completo.
//!
//! La GTK que va dentro de la AppImage aborta al arrancar en Sway, Hyprland y
//! parecidos cuando el tema de cursor de GSettings está vacío. Allí GSettings no
//! guarda ajustes que MNEMOS necesite, así que la app se vuelve a ejecutar a sí
//! misma con `GSETTINGS_BACKEND=memory` antes de que GTK arranque. En GNOME, KDE
//! y demás escritorios que sí usan GSettings no se toca nada, y si el usuario ya
//! ha fijado la variable, se respeta. Fuera de la AppImage tampoco se hace nada.

use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::process::Command;

/// Escritorios que guardan sus ajustes en GSettings (valores de
/// `XDG_CURRENT_DESKTOP`, que puede llevar varios separados por `:`).
const ESCRITORIOS_CON_GSETTINGS: &[&str] = &[
    "GNOME", "KDE", "Cinnamon", "XFCE", "MATE", "Budgie", "Unity", "Pantheon", "Deepin", "COSMIC",
    "LXQt", "LXDE",
];

/// Decide si hay que forzar `GSETTINGS_BACKEND=memory`, leyendo las variables
/// de entorno con `var`.
fn necesita_gsettings_en_memoria(var: impl Fn(&str) -> Option<OsString>) -> bool {
    let definida = |nombre: &str| var(nombre).is_some_and(|v| !v.is_empty());
    if !definida("APPIMAGE") || !definida("WAYLAND_DISPLAY") || definida("GSETTINGS_BACKEND") {
        return false;
    }
    let escritorio = var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let escritorio = escritorio.to_string_lossy();
    !escritorio
        .split(':')
        .any(|e| ESCRITORIOS_CON_GSETTINGS.iter().any(|c| e.contains(c)))
}

/// Si hace falta, vuelve a ejecutar MNEMOS con `GSETTINGS_BACKEND=memory`. Hay
/// que llamarla lo primero, antes de que arranque GTK. Si la nueva ejecución no
/// se puede lanzar, sigue como si nada.
pub fn preparar() {
    if !necesita_gsettings_en_memoria(|n| std::env::var_os(n)) {
        return;
    }
    let Ok(ejecutable) = std::env::current_exe() else {
        return;
    };
    let error = Command::new(ejecutable)
        .args(std::env::args_os().skip(1))
        .env("GSETTINGS_BACKEND", "memory")
        .exec();
    eprintln!("MNEMOS: no se pudo relanzar con GSETTINGS_BACKEND=memory: {error}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decide(vars: &[(&str, &str)]) -> bool {
        necesita_gsettings_en_memoria(|n| {
            vars.iter()
                .find(|(k, _)| *k == n)
                .map(|(_, v)| OsString::from(v))
        })
    }

    const APPIMAGE: (&str, &str) = ("APPIMAGE", "/media/u/FOTOS/MNEMOS-linux.AppImage");
    const WAYLAND: (&str, &str) = ("WAYLAND_DISPLAY", "wayland-1");

    #[test]
    fn en_sway_desde_la_appimage_se_fuerza() {
        assert!(decide(&[
            APPIMAGE,
            WAYLAND,
            ("XDG_CURRENT_DESKTOP", "sway")
        ]));
        assert!(decide(&[
            APPIMAGE,
            WAYLAND,
            ("XDG_CURRENT_DESKTOP", "Hyprland")
        ]));
        assert!(decide(&[APPIMAGE, WAYLAND]));
    }

    #[test]
    fn en_escritorios_con_gsettings_no() {
        for escritorio in ["ubuntu:GNOME", "KDE", "X-Cinnamon", "Budgie:GNOME", "XFCE"] {
            assert!(
                !decide(&[APPIMAGE, WAYLAND, ("XDG_CURRENT_DESKTOP", escritorio)]),
                "{escritorio}"
            );
        }
    }

    #[test]
    fn sin_appimage_sin_wayland_o_con_la_variable_no() {
        assert!(!decide(&[WAYLAND, ("XDG_CURRENT_DESKTOP", "sway")]));
        assert!(!decide(&[
            APPIMAGE,
            ("DISPLAY", ":0"),
            ("XDG_CURRENT_DESKTOP", "i3")
        ]));
        assert!(!decide(&[APPIMAGE, ("WAYLAND_DISPLAY", "")]));
        assert!(!decide(&[
            APPIMAGE,
            WAYLAND,
            ("GSETTINGS_BACKEND", "dconf")
        ]));
        assert!(!decide(&[
            APPIMAGE,
            WAYLAND,
            ("GSETTINGS_BACKEND", "memory")
        ]));
    }
}
