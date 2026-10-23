//! `mnemos`: herramienta de línea de órdenes para gestionar bibliotecas.

use std::path::PathBuf;
use std::process::ExitCode;

use mnemos_core::{Library, LibraryInfo};

const USAGE: &str = "Uso:
  mnemos init <ruta>   Crea una biblioteca vacía en <ruta>
  mnemos info <ruta>   Muestra el resumen de la biblioteca en <ruta>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [cmd, path] if cmd == "init" => {
            Library::create(&PathBuf::from(path)).and_then(|lib| lib.info())
        }
        [cmd, path] if cmd == "info" => {
            Library::open(&PathBuf::from(path)).and_then(|lib| lib.info())
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(info) => {
            print_info(&info);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn print_info(info: &LibraryInfo) {
    println!("Biblioteca: {}", info.root);
    println!("Esquema:    v{}", info.schema_version);
    println!("Fotos:      {}", info.photos);
    println!("Álbumes:    {}", info.albums);
    println!("Etiquetas:  {}", info.tags);
}
