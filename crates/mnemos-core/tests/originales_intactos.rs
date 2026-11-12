//! Prueba de punta a punta de la spec 009: editar no toca nunca un original.
//!
//! Recorre lo que hace la interfaz (guardar como original, cambiar de versión,
//! volver al original, guardar como copia, miniaturas, visor y volver a
//! indexar) y comprueba al final que cada archivo de `Originales/` tiene la
//! misma huella BLAKE3 y la misma fecha de modificación que al principio.

// Para que clippy trate también las funciones auxiliares como código de prueba.
#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use mnemos_core::Library;
use mnemos_core::library::{DATA_DIR, ORIGINALS_DIR};
use mnemos_imaging::recipe::{Crop, Geometry};

/// Huella y fecha de modificación de cada archivo de `Originales/`.
type Estado = BTreeMap<PathBuf, (String, SystemTime)>;

fn estado(root: &Path) -> Estado {
    let originales = root.join(ORIGINALS_DIR);
    walk(&originales)
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).unwrap();
            let modificado = fs::metadata(&path).unwrap().modified().unwrap();
            let relativa = path.strip_prefix(&originales).unwrap().to_owned();
            (
                relativa,
                (blake3::hash(&bytes).to_hex().to_string(), modificado),
            )
        })
        .collect()
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut archivos = Vec::new();
    for entrada in fs::read_dir(dir).unwrap() {
        let path = entrada.unwrap().path();
        if path.is_dir() {
            archivos.extend(walk(&path));
        } else {
            archivos.push(path);
        }
    }
    archivos
}

fn id_de(lib: &Library, nombre: &str) -> i64 {
    lib.connection()
        .query_row(
            "SELECT id FROM photos WHERE rel_path LIKE '%' || ?1",
            [nombre],
            |r| r.get(0),
        )
        .unwrap()
}

/// Biblioteca con las imágenes de `tests/fixtures` y una foto generada más
/// grande, en una subcarpeta con acentos.
fn biblioteca() -> (tempfile::TempDir, Library) {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = Library::create(dir.path()).unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let destino = dir.path().join(ORIGINALS_DIR).join("2025").join("Año ñ");
    fs::create_dir_all(&destino).unwrap();
    for nombre in ["cmyk.jpg", "fecha-exif.jpg"] {
        fs::copy(fixtures.join(nombre), destino.join(nombre)).unwrap();
    }
    image::RgbImage::from_fn(640, 480, |x, y| {
        // Cuadrantes de colores: cualquier giro o recorte cambia el resultado.
        match (x < 320, y < 240) {
            (true, true) => image::Rgb([220, 40, 40]),
            (false, true) => image::Rgb([40, 200, 60]),
            (true, false) => image::Rgb([40, 60, 220]),
            (false, false) => image::Rgb([230, 210, 40]),
        }
    })
    .save_with_format(destino.join("playa.jpg"), image::ImageFormat::Jpeg)
    .unwrap();
    lib.index().unwrap();
    lib.generate_thumbnails().unwrap();
    (dir, lib)
}

fn giro() -> Geometry {
    Geometry {
        rotate: 90,
        ..Geometry::default()
    }
}

fn recorte_y_volteo() -> Geometry {
    Geometry {
        rotate: 270,
        flip_horizontal: true,
        crop: Some(Crop {
            x: 0.1,
            y: 0.2,
            width: 0.6,
            height: 0.5,
        }),
    }
}

/// Lo que enseña el visor para la versión que se ve.
fn visor(lib: &Library, id: i64) -> Vec<u8> {
    let media = lib.photo_media(id).unwrap().unwrap();
    let path = media.rel_path.to_path(lib.root());
    let geometria = media.edit.map(|e| e.geometry).unwrap_or_default();
    mnemos_imaging::display_image_edited(&path, media.orientation, &geometria)
        .unwrap()
        .jpeg
}

#[test]
fn editar_no_toca_ningun_original() {
    let (dir, mut lib) = biblioteca();
    let root = dir.path().to_owned();
    let antes = estado(&root);
    assert_eq!(antes.len(), 3);

    let playa = id_de(&lib, "playa.jpg");
    let cmyk = id_de(&lib, "cmyk.jpg");

    // Guardar como original, dos versiones, y miniaturas editadas.
    assert_eq!(lib.save_edit(playa, giro()).unwrap(), 1);
    assert_eq!(lib.save_edit(playa, recorte_y_volteo()).unwrap(), 2);
    assert_eq!(lib.save_edit(cmyk, giro()).unwrap(), 1);
    lib.generate_thumbnails().unwrap();
    let version_2 = visor(&lib, playa);
    assert_ne!(version_2, visor(&lib, cmyk));

    // Volver al original y luego a la versión 2 da exactamente la versión 2.
    lib.set_edit_version(playa, None).unwrap();
    lib.generate_thumbnails().unwrap();
    assert!(lib.photo_media(playa).unwrap().unwrap().edit.is_none());
    assert_ne!(visor(&lib, playa), version_2);
    lib.set_edit_version(playa, Some(2)).unwrap();
    lib.generate_thumbnails().unwrap();
    assert_eq!(visor(&lib, playa), version_2);
    lib.set_edit_version(playa, Some(1)).unwrap();

    // Guardar como copia añade un archivo nuevo y no cambia los que había.
    let copia = lib.save_edit_as_copy(playa, &recorte_y_volteo()).unwrap();
    assert_ne!(copia, playa);
    lib.generate_thumbnails().unwrap();

    // Volver a indexar no cambia nada de lo anterior.
    lib.index().unwrap();

    let despues = estado(&root);
    assert_eq!(despues.len(), antes.len() + 1, "solo se añade la copia");
    for (ruta, huella) in &antes {
        assert_eq!(despues.get(ruta), Some(huella), "{} cambió", ruta.display());
    }
    // Las recetas viven fuera de `Originales/`.
    assert!(
        walk(&root.join(DATA_DIR).join("ediciones"))
            .iter()
            .all(|p| p.extension().is_some_and(|e| e == "json"))
    );
}

#[test]
fn las_ediciones_sobreviven_a_mover_la_biblioteca_y_borrar_la_base() {
    let (dir, mut lib) = biblioteca();
    let playa = id_de(&lib, "playa.jpg");
    lib.save_edit(playa, giro()).unwrap();
    lib.save_edit(playa, recorte_y_volteo()).unwrap();
    lib.set_edit_version(playa, Some(1)).unwrap();
    let antes = estado(dir.path());
    let esperado = lib.edit_summary(playa).unwrap();
    drop(lib);

    // Copiar la biblioteca a otra ruta, sin la base de datos.
    let otra = tempfile::tempdir().unwrap();
    for path in walk(dir.path()) {
        let relativa = path.strip_prefix(dir.path()).unwrap();
        // `mnemos.db` y sus archivos `-wal` y `-shm`.
        let nombre = relativa.file_name().unwrap().to_string_lossy();
        if relativa.starts_with(DATA_DIR) && nombre.starts_with("mnemos.db") {
            continue;
        }
        let destino = otra.path().join(relativa);
        fs::create_dir_all(destino.parent().unwrap()).unwrap();
        fs::copy(&path, &destino).unwrap();
    }
    let huellas = |raiz: &Path| -> Vec<_> {
        estado(raiz)
            .into_iter()
            .map(|(ruta, (huella, _))| (ruta, huella))
            .collect()
    };
    assert_eq!(
        huellas(otra.path()),
        huellas(dir.path()),
        "la copia tiene los mismos originales"
    );

    let mut lib = Library::create(otra.path()).unwrap();
    lib.index().unwrap();
    let playa = id_de(&lib, "playa.jpg");
    let resumen = lib.edit_summary(playa).unwrap();
    assert_eq!(resumen.current, Some(1));
    assert_eq!(resumen.versions, esperado.versions);
    assert_eq!(
        lib.photo_media(playa)
            .unwrap()
            .unwrap()
            .edit
            .map(|e| e.version),
        Some(1)
    );
    // Y la biblioteca de antes sigue igual.
    assert_eq!(estado(dir.path()), antes);
}
