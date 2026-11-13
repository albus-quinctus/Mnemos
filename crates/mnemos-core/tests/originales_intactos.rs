//! Spec 006, tarea 14: favoritos, archivo, títulos de día y selección no
//! tocan los originales. Cada acción se comprueba contra la huella BLAKE3, el
//! tamaño y la fecha de modificación de todos los archivos de la biblioteca
//! fuera de `.mnemos/`.
//!
//! Spec 015, tarea 11: enviar a la Papelera y restaurar tampoco los tocan, y
//! eliminar para siempre borra solo los originales de las fotos de la
//! Papelera, nunca otro archivo.
//!
//! Spec 014, tarea 8: crear, renombrar, llenar, vaciar y eliminar álbumes
//! solo cambia la base de datos; los nombres nunca llegan al disco.

#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use mnemos_core::{Library, Scope, SearchQuery, TimelineCursor, View};
use mnemos_imaging::recipe::Geometry;

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

    // Papelera: una foto editada, favorita y archivada vuelve igual.
    let giro = Geometry {
        rotate: 90,
        ..Geometry::default()
    };
    assert_eq!(lib.save_edit(ids[0], giro).unwrap(), 1);
    lib.set_favorite(&ids[..1], true).unwrap();
    lib.set_archived(&ids[..1], true).unwrap();
    lib.send_to_trash(&ids[..1]).unwrap();
    lib.restore_from_trash(&ids[..1]).unwrap();
    assert_eq!(lib.edit_summary(ids[0]).unwrap().current, Some(1));
    let archivada = lib.view_page(View::Archive, None, 100).unwrap().photos;
    assert!(archivada.iter().any(|f| f.id == ids[0] && f.favorite));
    lib.set_archived(&ids[..1], false).unwrap();
    lib.set_favorite(&ids[..1], false).unwrap();
    comprobar("restaurar una foto editada");

    // Papelera: enviar, ver y restaurar. Eliminar para siempre fotos que no
    // están en la Papelera, o vaciar sin nada caducado, no borra nada.
    let cambiadas = lib.send_to_trash(&ids).unwrap();
    assert_eq!(cambiadas.len(), ids.len());
    comprobar("enviar a la Papelera");
    lib.restore_from_trash(&cambiadas).unwrap();
    comprobar("restaurar");
    let informe = lib.delete_permanently(&ids, |_, _| {}).unwrap();
    assert!(informe.deleted.is_empty() && informe.failed.is_empty());
    let ahora = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    lib.empty_expired_trash(ahora, false, |_, _| {}).unwrap();
    comprobar("eliminar o vaciar fuera de la Papelera");

    // Álbumes: crear (con un nombre que sería mala carpeta), añadir, quitar,
    // renombrar y eliminar. Una foto archivada sigue en su álbum.
    let album = lib.create_album("Viaje / «Picos» 🏔️").unwrap();
    assert_eq!(lib.add_to_album(album, &ids).unwrap(), ids);
    comprobar("añadir a un álbum");
    lib.set_archived(&ids[..1], true).unwrap();
    let dentro = lib.view_page(View::Album(album), None, 100).unwrap().photos;
    assert_eq!(dentro.len(), ids.len());
    lib.set_archived(&ids[..1], false).unwrap();
    lib.remove_from_album(album, &ids[..1]).unwrap();
    comprobar("quitar de un álbum");
    lib.rename_album(album, "..").unwrap();
    comprobar("renombrar un álbum");
    lib.delete_album(album).unwrap();
    assert!(lib.albums().unwrap().is_empty());
    assert_eq!(
        lib.view_page(View::Photos, None, 100).unwrap().photos.len(),
        ids.len(),
        "eliminar un álbum no quita fotos de la biblioteca"
    );
    comprobar("eliminar un álbum");

    // Etiquetas (spec 016): crear (con operadores de FTS5 en el nombre),
    // poner, quitar, renombrar, unir y borrar. Solo cambia la base de datos;
    // borrar una etiqueta no quita fotos.
    let playa = lib.create_tag("Playa \"AND\" * / 🏖️").unwrap();
    let viaje = lib.create_tag("Viaje").unwrap();
    assert_eq!(lib.add_tag(&ids, playa.id).unwrap(), ids);
    lib.add_tag(&ids[..1], viaje.id).unwrap();
    comprobar("poner etiquetas");
    lib.set_archived(&ids[..1], true).unwrap();
    let dentro = lib
        .view_page(View::Tag(playa.id), None, 100)
        .unwrap()
        .photos;
    assert_eq!(dentro.len(), ids.len());
    lib.set_archived(&ids[..1], false).unwrap();
    lib.tag_usage(&ids).unwrap();
    lib.photo_tags(ids[0]).unwrap();
    lib.remove_tag(&ids[..1], playa.id).unwrap();
    comprobar("quitar una etiqueta");
    lib.rename_tag(playa.id, "..").unwrap();
    comprobar("renombrar una etiqueta");
    lib.merge_tags(viaje.id, playa.id).unwrap();
    comprobar("unir etiquetas");
    lib.delete_tag(playa.id).unwrap();
    assert!(lib.tags().unwrap().is_empty());
    assert_eq!(
        lib.view_page(View::Photos, None, 100).unwrap().photos.len(),
        ids.len(),
        "borrar una etiqueta no quita fotos de la biblioteca"
    );
    comprobar("borrar una etiqueta");

    // Ver las vistas.
    for vista in [View::Photos, View::Favorites, View::Archive, View::Trash] {
        lib.view_months(vista).unwrap();
        lib.view_page(vista, None, 100).unwrap();
    }
    comprobar("ver Fotos, Favoritos, Archivo y Papelera");

    // Al cerrar y volver a abrir, todo sigue igual.
    drop(lib);
    Library::open(raiz.path()).unwrap();
    comprobar("cerrar y abrir");
}

#[test]
fn eliminar_para_siempre_solo_borra_lo_de_la_papelera() {
    let (_origen, raiz, mut lib, fotos) = biblioteca();
    let ids: Vec<i64> = fotos.iter().map(|f| f.0).collect();
    let antes = estado(raiz.path());

    // Una foto a la Papelera; se pide eliminar todas.
    lib.send_to_trash(&ids[..1]).unwrap();
    let en_papelera = lib.trash_entries().unwrap();
    assert_eq!(en_papelera.len(), 1);
    let borrada = PathBuf::from(&en_papelera[0].rel_path);
    let informe = lib.delete_permanently(&ids, |_, _| {}).unwrap();
    assert_eq!(informe.deleted, vec![ids[0]]);
    assert!(informe.failed.is_empty());

    let mut esperado = antes.clone();
    assert!(esperado.remove(&borrada).is_some(), "no era un original");
    assert_eq!(estado(raiz.path()), esperado, "se ha borrado otro archivo");

    // Vaciado automático 31 días después: solo lo que está en la Papelera.
    lib.send_to_trash(&ids[1..2]).unwrap();
    let otra = PathBuf::from(&lib.trash_entries().unwrap()[0].rel_path);
    let dentro_de_31_dias = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + 31 * 24 * 3600;
    lib.empty_expired_trash(dentro_de_31_dias, true, |_, _| {})
        .unwrap();
    esperado.remove(&otra);
    assert_eq!(
        estado(raiz.path()),
        esperado,
        "el vaciado ha borrado otro archivo"
    );
    assert_eq!(
        lib.view_page(View::Photos, None, 100).unwrap().photos.len(),
        ids.len() - 2
    );
}
