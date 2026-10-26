//! `mnemos`: herramienta de línea de órdenes para gestionar bibliotecas.

use std::path::Path;
use std::process::ExitCode;

use mnemos_core::{
    FileError, ImportReport, IndexReport, Library, LibraryInfo, Result, ThumbnailReport,
};

const USAGE: &str = "Uso:
  mnemos init <biblioteca>            Crea una biblioteca vacía
  mnemos info <biblioteca>            Muestra el resumen de la biblioteca
  mnemos index <biblioteca>           Registra las fotos que hay en Originales/
  mnemos import <biblioteca> <origen> Copia las fotos de <origen> a la biblioteca
  mnemos miniaturas <biblioteca>      Genera las miniaturas que faltan";

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
