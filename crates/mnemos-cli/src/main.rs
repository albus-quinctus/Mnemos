//! `mnemos`: herramienta de línea de órdenes para gestionar bibliotecas.

use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::ExitCode;

use mnemos_core::duplicates::MAX_GROUPS_PAGE;
use mnemos_core::{
    DeleteReport, FileError, ImportReport, IndexReport, Library, LibraryInfo, Result,
    ThumbnailReport,
};

const USAGE: &str = "Uso:
  mnemos init <biblioteca>            Crea una biblioteca vacía
  mnemos info <biblioteca>            Muestra el resumen de la biblioteca
  mnemos index <biblioteca>           Registra las fotos que hay en Originales/
  mnemos import <biblioteca> <origen> Copia las fotos de <origen> a la biblioteca
  mnemos miniaturas <biblioteca>      Genera las miniaturas que faltan
  mnemos duplicados <biblioteca>      Lista las fotos repetidas (no borra nada)
  mnemos etiquetas <biblioteca>       Lista las etiquetas y cuántas fotos tienen
  mnemos papelera <biblioteca>        Lista la papelera y cuándo se elimina cada foto
  mnemos papelera vaciar <biblioteca> [--confirmar]
                                      Elimina PARA SIEMPRE las fotos de la papelera";

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
        ["etiquetas", lib] => Library::open(Path::new(lib)).and_then(|lib| print_tags(&lib)),
        ["papelera", "vaciar", lib] => empty_trash(Path::new(lib), false),
        ["papelera", "vaciar", lib, "--confirmar"] => empty_trash(Path::new(lib), true),
        ["papelera", lib] => Library::open(Path::new(lib)).and_then(|lib| print_trash(&lib)),
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
    if r.in_trash > 0 {
        println!("Ya estaban en la papelera: {}", r.in_trash);
    }
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
fn print_tags(lib: &Library) -> Result<bool> {
    let tags = lib.tags()?;
    if tags.is_empty() {
        println!("No hay etiquetas.");
        return Ok(true);
    }
    for tag in &tags {
        let fotos = if tag.photos == 1 { "foto" } else { "fotos" };
        println!("{:>7} {fotos:<5}  {}", tag.photos, tag.name);
    }
    Ok(true)
}

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

/// Lista la Papelera (spec 015). Solo consulta.
fn print_trash(lib: &Library) -> Result<bool> {
    let entries = lib.trash_entries()?;
    if entries.is_empty() {
        println!("La papelera está vacía.");
        return Ok(true);
    }
    println!("En la papelera: {}", entries.len());
    for e in &entries {
        println!("  {}  se elimina el {}", e.rel_path, e.deletes_on);
    }
    Ok(true)
}

/// Vacía la Papelera: borra del disco sus fotos. Pide escribir «sí» salvo
/// con `--confirmar`.
fn empty_trash(root: &Path, confirmed: bool) -> Result<bool> {
    let mut lib = Library::open(root)?;
    let ids = lib.trash_ids()?;
    if ids.is_empty() {
        println!("La papelera está vacía.");
        return Ok(true);
    }
    if !confirmed {
        print!(
            "Se borrarán del disco {} fotos y no se podrán recuperar. Escribe «sí» para seguir: ",
            ids.len()
        );
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().lock().read_line(&mut answer)?;
        if !matches!(answer.trim().to_lowercase().as_str(), "sí" | "si") {
            println!("No se ha borrado nada.");
            return Ok(true);
        }
    }
    let report = lib.delete_permanently(&ids, |_, _| {})?;
    Ok(print_delete(&report))
}

fn print_delete(r: &DeleteReport) -> bool {
    println!("Eliminadas: {}", r.deleted.len());
    if r.failed.is_empty() {
        return true;
    }
    println!(
        "No se pudieron eliminar (siguen en la papelera): {}",
        r.failed.len()
    );
    for f in &r.failed {
        eprintln!("  {}: {}", f.rel_path, f.reason);
    }
    false
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
