//! Spec 006, tarea 14: favoritos, archivo, títulos de día y selección no
//! tocan los originales. Cada acción se comprueba contra la huella BLAKE3, el
//! tamaño y la fecha de modificación de todos los archivos de la biblioteca
//! fuera de `.mnemos/`.

#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use mnemos_core::{Library, Scope, SearchQuery, TimelineCursor, View};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures");

type Estado = BTreeMap<PathBuf, (String, u64, SystemTime)>;

/// Huella, tamaño y fecha de modificación de cada archivo fuera de `.mnemos/`.
fn estado(raiz: &Path) -> Estado {
    let mut estado = Estado::new();
    let mut pendientes = vec![raiz.to_path_buf()];
    while let Some(dir) = pendientes.pop() {
        for entrada in fs::read_dir(&dir).unwrap() {
            let entrada = entrada.unwrap();
            let ruta = entrada.path();
            if ruta.file_name().is_some_and(|n| n == ".mnemos") {
                continue;
            }
            if entrada.file_type().unwrap().is_dir() {
                pendientes.push(ruta);
                continue;
            }
            let datos = fs::metadata(&ruta).unwrap();
            let huella = blake3::hash(&fs::read(&ruta).unwrap()).to_hex().to_string();
            estado.insert(
                ruta.strip_prefix(raiz).unwrap().to_path_buf(),
                (huella, datos.len(), datos.modified().unwrap()),
            );
        }
    }
    estado
}

/// Biblioteca con las fotos de `tests/fixtures`, importadas como lo haría un
/// usuario. Devuelve también los ids en el orden de la línea de tiempo.
fn biblioteca() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Library,
    Vec<(i64, i64)>,
) {
    let origen = tempfile::tempdir().unwrap();
    for entrada in fs::read_dir(FIXTURES).unwrap() {
        let ruta = entrada.unwrap().path();
        if ruta.extension().is_some_and(|e| e == "jpg") {
            fs::copy(&ruta, origen.path().join(ruta.file_name().unwrap())).unwrap();
        }
    }
    let raiz = tempfile::tempdir().unwrap();
    let mut lib = Library::create(raiz.path()).unwrap();
    lib.import_from(origen.path()).unwrap();
    let fotos = lib
        .view_page(View::Photos, None, 100)
        .unwrap()
        .photos
        .iter()
        .map(|f| (f.id, f.date))
        .collect::<Vec<_>>();
    assert!(fotos.len() >= 2, "hacen falta al menos dos fotos de prueba");
    (origen, raiz, lib, fotos)
}

#[test]
fn ninguna_accion_toca_los_originales() {
    let (_origen, raiz, mut lib, fotos) = biblioteca();
    let ids: Vec<i64> = fotos.iter().map(|f| f.0).collect();
    let antes = estado(raiz.path());
    assert!(!antes.is_empty());

    // Con la biblioteca aún abierta: el disco ya tiene que estar igual.
    let comprobar = |accion: &str| {
        assert_eq!(
            estado(raiz.path()),
            antes,
            "«{accion}» ha cambiado un original"
        );
    };

    // Seleccionar.
    let cursor = |(id, date): (i64, i64)| TimelineCursor { date, id };
    let busqueda = SearchQuery::parse("fecha");
    lib.select_all(Scope::View(View::Photos)).unwrap();
    lib.select_all(Scope::Search(&busqueda)).unwrap();
    lib.select_range(
        Scope::View(View::Photos),
        cursor(fotos[0]),
        cursor(fotos[1]),
    )
    .unwrap();
    comprobar("seleccionar");

    // Favoritos, marcar y deshacer.
    let cambiadas = lib.set_favorite(&ids, true).unwrap();
    comprobar("marcar favoritas");
    lib.set_favorite(&cambiadas, false).unwrap();
    comprobar("deshacer favoritas");

    // Archivo, archivar y deshacer.
    let cambiadas = lib.set_archived(&ids, true).unwrap();
    assert!(
        lib.view_page(View::Photos, None, 100)
            .unwrap()
            .photos
            .is_empty()
    );
    comprobar("archivar");
    lib.set_archived(&cambiadas, false).unwrap();
    comprobar("desarchivar");

    // Títulos de día: guardar, cambiar y borrar.
    lib.set_day_title("2024-10-12", "Ruta del Cares").unwrap();
    lib.set_day_title("2024-10-12", "Ruta del Cares 🏔️")
        .unwrap();
    lib.set_day_title("2024-10-12", "").unwrap();
    comprobar("títulos de día");

    // Ver las vistas.
    for vista in [View::Photos, View::Favorites, View::Archive] {
        lib.view_months(vista).unwrap();
        lib.view_page(vista, None, 100).unwrap();
    }
    comprobar("ver Fotos, Favoritos y Archivo");

    // Al cerrar y volver a abrir, todo sigue igual.
    drop(lib);
    Library::open(raiz.path()).unwrap();
    comprobar("cerrar y abrir");
}
