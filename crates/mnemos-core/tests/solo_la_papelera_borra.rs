//! Regla 1 de `CLAUDE.md` y su única excepción (spec 015): solo la Papelera
//! (`src/trash.rs`) borra originales.
//!
//! Recorre el código del núcleo, sin sus pruebas, y busca cada llamada que
//! borra o mueve archivos. Fuera de `trash.rs` solo valen las de esta lista:
//! temporales `.part`, cachés de `.mnemos/` y archivos que la propia app
//! acaba de crear. Si añades una, revisa que no pueda tocar un original y
//! apúntala aquí con el motivo.

use std::fs;
use std::path::Path;

/// (archivo, llamada) permitidas fuera de `trash.rs`, con su motivo.
const PERMITIDAS: &[(&str, &str, &str)] = &[
    (
        "copies.rs",
        "fs::remove_file(&part)",
        "temporal de la copia editada",
    ),
    (
        "copies.rs",
        "fs::rename(&part, &dest)",
        "temporal a la copia nueva",
    ),
    (
        "copies.rs",
        "fs::remove_file(&dest)",
        "copia nueva que no se pudo registrar",
    ),
    (
        "edits.rs",
        "fs::remove_file(path)",
        "miniaturas editadas viejas (caché)",
    ),
    (
        "edits.rs",
        "fs::rename(&part, &dest)",
        "receta temporal a la definitiva",
    ),
    ("edits.rs", "fs::remove_file(&part)", "receta temporal"),
    (
        "edits.rs",
        "fs::rename(&src, &dest)",
        "aparta una receta dañada",
    ),
    (
        "export.rs",
        "fs::remove_file(&dest)",
        "exportación fuera de la biblioteca",
    ),
    (
        "export.rs",
        "fs::remove_file(&path)",
        "exportación fuera de la biblioteca",
    ),
    (
        "import.rs",
        "fs::remove_file(&part)",
        "temporal de la importación",
    ),
    (
        "import.rs",
        "fs::rename(&part, &dest)",
        "temporal a la foto importada",
    ),
    (
        "thumbnails.rs",
        "fs::rename(&part, &dest)",
        "miniatura temporal (caché)",
    ),
    (
        "thumbnails.rs",
        "fs::remove_file(&part)",
        "miniatura temporal (caché)",
    ),
];

const LLAMADAS: &[&str] = &["remove_file(", "remove_dir(", "remove_dir_all(", "rename("];

#[test]
fn solo_la_papelera_borra_o_mueve_originales() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut encontradas = Vec::new();
    recorrer(&src, &src, &mut encontradas);

    let mut sin_permiso = Vec::new();
    for (archivo, linea) in &encontradas {
        if archivo == "trash.rs" {
            continue;
        }
        let permitida = PERMITIDAS
            .iter()
            .any(|(a, llamada, _)| a == archivo && linea.contains(llamada));
        if !permitida {
            sin_permiso.push(format!("{archivo}: {linea}"));
        }
    }
    assert!(
        sin_permiso.is_empty(),
        "código que borra o mueve archivos fuera de la Papelera:\n{}",
        sin_permiso.join("\n")
    );

    // Y la Papelera borra originales en un único sitio: `delete_original`.
    let papelera = fs::read_to_string(src.join("trash.rs")).unwrap();
    let codigo = sin_pruebas(&papelera);
    let originales: Vec<&str> = codigo
        .lines()
        .filter(|l| l.contains("fs::remove_file(&path)"))
        .collect();
    assert_eq!(originales.len(), 1, "{originales:?}");
    assert!(
        encontradas.iter().any(|(a, _)| a == "trash.rs"),
        "no se ha encontrado el borrado de la Papelera"
    );
}

fn recorrer(raiz: &Path, carpeta: &Path, encontradas: &mut Vec<(String, String)>) {
    for entrada in fs::read_dir(carpeta).expect("carpeta de código legible") {
        let ruta = entrada.expect("entrada legible").path();
        if ruta.is_dir() {
            recorrer(raiz, &ruta, encontradas);
            continue;
        }
        if ruta.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let nombre = ruta
            .strip_prefix(raiz)
            .expect("dentro de src")
            .to_string_lossy()
            .replace('\\', "/");
        let texto = fs::read_to_string(&ruta).expect("código en UTF-8");
        for linea in sin_pruebas(&texto).lines() {
            let linea = linea.trim();
            if linea.starts_with("//") {
                continue;
            }
            if LLAMADAS.iter().any(|l| linea.contains(l)) {
                encontradas.push((nombre.clone(), linea.to_owned()));
            }
        }
    }
}

/// El código sin los módulos de pruebas, que en este proyecto van siempre al
/// final del archivo.
fn sin_pruebas(texto: &str) -> &str {
    texto.split("#[cfg(test)]").next().unwrap_or(texto)
}
