//! Mide cuánto tarda en generarse la miniatura de cada foto de una carpeta.
//!
//! ```text
//! cargo run --release -p mnemos-imaging --example medir_miniaturas -- <carpeta> [repeticiones] [carpeta_salida]
//! ```
//!
//! Lee las fotos, nunca las modifica. Con `carpeta_salida` guarda cada
//! miniatura como `<nombre>.miniatura.jpg` para compararlas a ojo.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(dir) = args.next().map(PathBuf::from) else {
        eprintln!("uso: medir_miniaturas <carpeta> [repeticiones] [carpeta_salida]");
        std::process::exit(2);
    };
    let runs: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(5).max(1);
    let out = args.next().map(PathBuf::from);
    if let Some(out) = &out {
        std::fs::create_dir_all(out).expect("no se pudo crear la carpeta de salida");
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("no se pudo leer la carpeta")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && mnemos_imaging::is_supported_image(p))
        .collect();
    files.sort();

    println!(
        "{:<28} {:>11} {:>11} {:>11}",
        "foto", "mediana ms", "mín ms", "tamaño"
    );
    let mut total = Duration::ZERO;
    for path in &files {
        let orientation = mnemos_imaging::orientation(path).unwrap_or(1);
        let mut times = Vec::with_capacity(runs);
        let mut result = mnemos_imaging::thumbnail(path, orientation);
        for _ in 0..runs {
            let start = Instant::now();
            result = mnemos_imaging::thumbnail(path, orientation);
            times.push(start.elapsed());
        }
        times.sort();
        let median = times[times.len() / 2];
        total += median;
        let name = name(path);
        match result {
            Ok(thumb) => {
                println!(
                    "{name:<28} {:>11.1} {:>11.1} {:>11}",
                    ms(median),
                    ms(times[0]),
                    format!("{}x{}", thumb.width, thumb.height)
                );
                if let Some(out) = &out {
                    std::fs::write(out.join(format!("{name}.miniatura.jpg")), &thumb.jpeg)
                        .expect("no se pudo guardar la miniatura");
                }
            }
            Err(e) => println!("{name:<28} {:>11.1} {:>11}   error: {e}", ms(median), ""),
        }
    }
    println!(
        "{} fotos, suma de medianas {:.0} ms",
        files.len(),
        ms(total)
    );
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}
