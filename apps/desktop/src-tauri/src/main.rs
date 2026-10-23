// Evita una consola adicional en Windows en las versiones de publicación. NO QUITAR.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mnemos_desktop_lib::run()
}
