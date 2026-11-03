//! `mnemos`: herramienta de línea de órdenes para gestionar bibliotecas.

use std::path::Path;
use std::process::ExitCode;

use mnemos_core::duplicates::MAX_GROUPS_PAGE;
use mnemos_core::{
    FileError, ImportReport, IndexReport, Library, LibraryInfo, Result, ThumbnailReport,
};

const USAGE: &str = "Uso:
  mnemos init <biblioteca>            Crea una biblioteca vacía
  mnemos info <biblioteca>            Muestra el resumen de la biblioteca
  mnemos index <biblioteca>           Registra las fotos que hay en Originales/
  mnemos import <biblioteca> <origen> Copia las fotos de <origen> a la biblioteca
  mnemos miniaturas <biblioteca>      Genera las miniaturas que faltan
  mnemos duplicados <biblioteca>      Lista las fotos repetidas (no borra nada)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["init", lib] => Library::create(Path::new(lib)).and_then(|lib| print_info(&lib.info()?)),
        ["info", lib] => Library::open(Path::new(lib)).and_then(|lib| print_info(&lib.info()?)),
        ["index", lib] => Library::open(Path::new(lib))
            .and_then(|mut lib| lib.index())
            .map(|r| print_index(&r)),
        ["import", lib, source] => Library::open(Path::new(lib))
            .and_then(|mut lib| lib.import_from(Path::new(source)))
            .map(|r| print_import(&r)),
        ["miniaturas", lib] => Library::open(Path::new(lib))
            .and_then(|lib| lib.generate_thumbnails())
            .map(|r| print_thumbnails(&r)),
        ["duplicados", lib] => Library::open(Path::new(lib)).and_then(|lib| print_duplicates(&lib)),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        // La operación terminó, pero algunos archivos no se pudieron procesar.
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn print_info(info: &LibraryInfo) -> Result<bool> {
    println!("Biblioteca: {}", info.root);
    println!("Esquema:    v{}", info.schema_version);
    println!("Fotos:      {}", info.photos);
    println!("Álbumes:    {}", info.albums);
    println!("Etiquetas:  {}", info.tags);
    Ok(true)
}

fn print_index(r: &IndexReport) -> bool {
    println!("Nuevas:        {}", r.added);
    println!("Actualizadas:  {}", r.updated);
    println!("Sin cambios:   {}", r.unchanged);
    println!("Desaparecidas: {}", r.missing);
    print_errors(&r.errors)
}

fn print_import(r: &ImportReport) -> bool {
    println!("Copiadas:   {}", r.copied);
    println!("Duplicadas: {}", r.duplicates);
    print_errors(&r.errors)
}

fn print_thumbnails(r: &ThumbnailReport) -> bool {
    println!("Generadas:      {}", r.written);
    println!("Sin miniatura:  {}", r.failed.len());
    for (path, message) in &r.failed {
        eprintln!("  {path}: {message}");
    }
    // Un formato sin miniatura no es un error de la orden.
    true
}

/// Lista los grupos de fotos repetidas. Solo consulta: no toca nada.
fn print_duplicates(lib: &Library) -> Result<bool> {
    let s = lib.duplicate_summary()?;
    if s.groups == 0 {
        println!("No hay fotos repetidas.");
        return Ok(true);
    }
    println!("Grupos:          {}", s.groups);
    println!("Copias de más:   {}", s.extra_copies);
    println!("Espacio de más:  {}", human_bytes(s.extra_bytes));
    let mut cursor = None;
    loop {
        let page = lib.duplicate_groups(cursor.as_ref(), MAX_GROUPS_PAGE)?;
        for g in &page.groups {
            let size = g.copies.first().map_or(0, |c| c.size_bytes);
            println!();
            println!(
                "{} copias · {} · {}",
                g.copies.len(),
                human_bytes(size),
                &g.content_hash[..g.content_hash.len().min(12)]
            );
            for c in &g.copies {
                let mut notes = Vec::new();
                if c.favorite {
                    notes.push("favorita");
                }
                if c.archived {
                    notes.push("archivada");
                }
                if !c.exists {
                    notes.push("desaparecida");
                }
                if notes.is_empty() {
                    println!("  {}", c.rel_path);
                } else {
                    println!("  {} ({})", c.rel_path, notes.join(", "));
                }
            }
        }
        match page.next {
            Some(next) => cursor = Some(next),
            None => return Ok(true),
        }
    }
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit]).replace('.', ",")
    }
}

/// Muestra los errores por archivo. Devuelve `true` si no hubo ninguno.
fn print_errors(errors: &[FileError]) -> bool {
    if !errors.is_empty() {
        eprintln!("Errores:    {}", errors.len());
        for e in errors {
            eprintln!("  {}: {}", e.path, e.message);
        }
    }
    errors.is_empty()
}
