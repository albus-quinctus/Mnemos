//! Mide cuánto tarda la imagen del visor y la miniatura de una foto editada
//! (spec 009) frente a la misma foto sin editar.
//!
//! ```text
//! cargo run --release -p mnemos-imaging --example medir_ediciones -- [foto] [repeticiones]
//! ```
//!
//! Sin `foto`, genera una foto sintética de 6000 × 4000 (24 MP) en una carpeta
//! temporal. Lee la foto, nunca la modifica.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use mnemos_imaging::recipe::{Crop, Geometry};
use mnemos_imaging::{Thumbnail, ThumbnailError};

fn main() {
    let mut args = std::env::args().skip(1);
    let given = args.next().map(PathBuf::from);
    let runs: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(5).max(1);
    let tmp = std::env::temp_dir().join(format!("mnemos-medir-ediciones-{}", std::process::id()));
    let path = given.clone().unwrap_or_else(|| {
        std::fs::create_dir_all(&tmp).expect("no se pudo crear la carpeta temporal");
        let path = tmp.join("sintetica-24mp.jpg");
        image::RgbImage::from_fn(6000, 4000, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        })
        .save(&path)
        .expect("no se pudo escribir la foto sintética");
        path
    });
    let orientation = mnemos_imaging::orientation(&path).unwrap_or(1);

    let cases = [
        ("sin editar", Geometry::default()),
        (
            "giro 90°",
            Geometry {
                rotate: 90,
                ..Geometry::default()
            },
        ),
        (
            "giro y recorte 80 %",
            Geometry {
                rotate: 90,
                flip_horizontal: false,
                crop: Some(Crop {
                    x: 0.1,
                    y: 0.1,
                    width: 0.8,
                    height: 0.8,
                }),
            },
        ),
        (
            "recorte 25 %",
            Geometry {
                crop: Some(Crop {
                    x: 0.4,
                    y: 0.4,
                    width: 0.25,
                    height: 0.25,
                }),
                ..Geometry::default()
            },
        ),
    ];
    println!(
        "{:<22} {:>12} {:>14}",
        "edición", "visor ms", "miniatura ms"
    );
    for (name, g) in &cases {
        let view = median(runs, || {
            mnemos_imaging::display_image_edited(&path, orientation, g)
        });
        let thumb = median(runs, || {
            mnemos_imaging::thumbnail_edited(&path, orientation, g)
        });
        println!("{name:<22} {:>12.1} {:>14.1}", ms(view), ms(thumb));
    }
    if given.is_none() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

fn median(runs: usize, f: impl Fn() -> Result<Thumbnail, ThumbnailError>) -> Duration {
    f().expect("no se pudo generar la imagen");
    let mut times: Vec<Duration> = (0..runs)
        .map(|_| {
            let start = Instant::now();
            f().expect("no se pudo generar la imagen");
            start.elapsed()
        })
        .collect();
    times.sort();
    times[times.len() / 2]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}
