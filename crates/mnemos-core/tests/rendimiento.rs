//! Prueba de rendimiento de la spec 002 con una biblioteca sintética.
//!
//! No corre con `cargo test` normal porque tarda. Se lanza en modo release:
//!
//! ```text
//! cargo test --release -p mnemos-core --test rendimiento -- --ignored --nocapture
//! ```
//!
//! `MNEMOS_FOTOS` cambia el número de fotos (10 000 por defecto).

// Para que clippy trate también las funciones auxiliares como código de prueba.
#![cfg(test)]

use std::fs::{self, File};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use mnemos_core::thumbnails::DEFAULT_WORKERS;
use mnemos_core::{Library, ThumbnailEvent, ThumbnailQueue};

const PAGINA: u32 = 500;
// Tres años hacia atrás desde el 26-10-2026.
const AHORA: u64 = 1_793_000_000;
const TRES_ANOS: u64 = 3 * 365 * 86_400;

/// Proporciones de cámara y de móvil, en horizontal y en vertical.
const TAMANOS: [(u32, u32); 5] = [(800, 600), (600, 800), (800, 533), (1024, 512), (450, 800)];

fn crear_fotos(carpeta: &Path, total: usize) {
    for i in 0..total {
        let (w, h) = TAMANOS[i % TAMANOS.len()];
        // Cada foto tiene un contenido distinto para que no haya duplicados.
        let (r, g, b) = (
            (i % 251) as u8,
            ((i / 251) % 251) as u8,
            (i * 7 % 256) as u8,
        );
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([
                r.wrapping_add((x / 8) as u8),
                g.wrapping_add((y / 8) as u8),
                b,
            ])
        });
        let ruta = carpeta.join(format!("{:03}/IMG_{i:05}.jpg", i / 1000));
        fs::create_dir_all(ruta.parent().unwrap()).unwrap();
        img.save_with_format(&ruta, image::ImageFormat::Jpeg)
            .unwrap();
        let fecha = AHORA - (i as u64 * 7919) % TRES_ANOS;
        File::options()
            .write(true)
            .open(&ruta)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(fecha))
            .unwrap();
    }
}

fn medir<T>(nombre: &str, f: impl FnOnce() -> T) -> (T, Duration) {
    let inicio = Instant::now();
    let resultado = f();
    let tiempo = inicio.elapsed();
    println!("{nombre:<40} {tiempo:>10.2?}");
    (resultado, tiempo)
}

#[test]
#[ignore = "lenta: se lanza a mano en modo release"]
fn biblioteca_de_10000_fotos() {
    let total: usize = std::env::var("MNEMOS_FOTOS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(10_000);
    let dir = tempfile::tempdir().unwrap();
    let raiz = dir.path().join("Biblioteca");
    let mut lib = Library::create(&raiz).unwrap();

    println!("\n{total} fotos sintéticas");
    medir("crear las fotos (no cuenta)", || {
        crear_fotos(&raiz.join("Originales"), total)
    });

    let (informe, _) = medir("indexar", || lib.index().unwrap());
    assert_eq!(informe.added as usize, total);
    assert!(informe.errors.is_empty(), "{:?}", informe.errors);

    let (informe, _) = medir("volver a indexar sin cambios", || lib.index().unwrap());
    assert_eq!(informe.unchanged as usize, total);

    // Miniaturas con la misma cola y los mismos hilos que la aplicación.
    let ((), _) = medir(&format!("miniaturas ({DEFAULT_WORKERS} hilos)"), || {
        let trabajos = lib.pending_thumbnails().unwrap();
        assert_eq!(trabajos.len(), total);
        let (tx, rx) = mpsc::channel();
        let cola = ThumbnailQueue::start(&raiz, DEFAULT_WORKERS, move |evento| {
            let _ = tx.send(evento);
        })
        .unwrap();
        cola.push(trabajos);
        let mut hechas = 0;
        while hechas < total {
            match rx.recv_timeout(Duration::from_secs(60)).unwrap() {
                ThumbnailEvent::Ready { photo_ids } => hechas += photo_ids.len(),
                ThumbnailEvent::Failed { message, .. } => panic!("miniatura fallida: {message}"),
                ThumbnailEvent::Stopped { message } => panic!("cola parada: {message}"),
            }
        }
    });
    assert!(lib.pending_thumbnails().unwrap().is_empty());

    let (meses, tiempo_meses) = medir("meses de la barra lateral", || {
        lib.timeline_months().unwrap()
    });
    assert_eq!(
        meses.iter().map(|m| m.photos as usize).sum::<usize>(),
        total
    );

    // Recorrer la cuadrícula entera, página a página, como al desplazarse.
    let mut peor = Duration::ZERO;
    let mut vistas = 0;
    let mut cursor = None;
    let ((), _) = medir(&format!("recorrer todo en páginas de {PAGINA}"), || {
        loop {
            let inicio = Instant::now();
            let pagina = lib.timeline_page(cursor, PAGINA).unwrap();
            peor = peor.max(inicio.elapsed());
            vistas += pagina.photos.len();
            match pagina.next {
                Some(siguiente) => cursor = Some(siguiente),
                None => break,
            }
        }
    });
    println!("{:<40} {peor:>10.2?}", "  página más lenta");
    assert_eq!(vistas, total);

    let (_, tiempo_detalles) = medir("detalles de una foto (visor)", || {
        lib.photo_details(total as i64 / 2).unwrap().unwrap()
    });

    let miniaturas = raiz.join(".mnemos/miniaturas");
    let ocupado: u64 = walk_size(&miniaturas);
    println!(
        "{:<40} {:>10.1} MB",
        "espacio de las miniaturas",
        ocupado as f64 / 1_048_576.0
    );

    // Límites holgados: lo que importa es que no crezcan con la biblioteca.
    assert!(
        tiempo_meses < Duration::from_millis(200),
        "{tiempo_meses:?}"
    );
    assert!(peor < Duration::from_millis(100), "{peor:?}");
    assert!(
        tiempo_detalles < Duration::from_millis(20),
        "{tiempo_detalles:?}"
    );
}

fn walk_size(dir: &Path) -> u64 {
    fs::read_dir(dir)
        .map(|entradas| {
            entradas
                .flatten()
                .map(|e| match e.file_type() {
                    Ok(t) if t.is_dir() => walk_size(&e.path()),
                    _ => e.metadata().map(|m| m.len()).unwrap_or(0),
                })
                .sum()
        })
        .unwrap_or(0)
}
