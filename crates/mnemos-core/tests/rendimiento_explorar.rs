//! Medición de Explorar (spec 021) con 10 000 fotos repartidas en 15 años y
//! 50 etiquetas.
//!
//! Las fotos son solo filas de la base de datos: Explorar no abre ningún
//! archivo. No corre con `cargo test` normal. Se lanza en modo release:
//!
//! ```text
//! cargo test --release -p mnemos-core --test rendimiento_explorar -- --ignored --nocapture
//! ```

#![cfg(test)]

use std::time::{Duration, Instant};

use mnemos_core::Library;
use rusqlite::params;

const FOTOS: i64 = 10_000;
const DIA: i64 = 86_400;
/// 2026-11-16 12:00 UTC.
const HOY: i64 = 1_794_830_400;

#[test]
#[ignore]
fn explorar_con_10_000_fotos() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = Library::create(dir.path()).unwrap();
    {
        let conn = lib.connection();
        conn.execute_batch("BEGIN").unwrap();
        for i in 1..=FOTOS {
            // Repartidas a lo largo de 15 años; algunas capturas, panorámicas,
            // vídeos, RAW, editadas, favoritas y archivadas.
            let taken = HOY - (i * 473_053) % (15 * 365 * DIA);
            let rel = match i % 50 {
                0 => format!("Originales/x/Screenshot_{i}.png"),
                1 => format!("Originales/x/DSC{i:05}.ARW"),
                2 => format!("Originales/x/VID_{i}.mp4"),
                _ => format!("Originales/x/IMG_{i:05}.jpg"),
            };
            let (w, h) = if i % 97 == 0 {
                (8000, 3000)
            } else {
                (4000, 3000)
            };
            conn.execute(
                "INSERT INTO photos (id, rel_path, size_bytes, modified_at, taken_at, imported_at,
                                     width, height, media_kind, edit_version, favorite,
                                     archived_at)
                 VALUES (?1, ?2, 1, ?3, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    i,
                    rel,
                    taken,
                    HOY - (i % 30) * DIA,
                    w,
                    h,
                    if i % 50 == 2 { "video" } else { "foto" },
                    (i % 61 == 0).then_some(1),
                    i % 13 == 0,
                    (i % 113 == 0).then_some(HOY),
                ],
            )
            .unwrap();
        }
        conn.execute_batch("COMMIT").unwrap();
    }
    for t in 0..50 {
        let tag = lib.create_tag(&format!("Etiqueta {t}")).unwrap();
        let ids: Vec<i64> = (1..=FOTOS).filter(|i| i % 50 == t).collect();
        lib.add_tag(&ids, tag.id).unwrap();
    }

    // La primera vez SQLite aún prepara y lee páginas: se mide la segunda.
    lib.explore(HOY).unwrap();
    let inicio = Instant::now();
    lib.tags().unwrap();
    println!("tags: {:?}", inicio.elapsed());
    for c in mnemos_core::Collection::ALL {
        let inicio = Instant::now();
        lib.view_months(mnemos_core::timeline::View::Collection(c))
            .unwrap();
        println!("{c:?}: {:?}", inicio.elapsed());
    }
    let inicio = Instant::now();
    let e = lib.explore(HOY).unwrap();
    let t = inicio.elapsed();
    println!(
        "explore: {:.1} ms ({} recuerdos, {} colecciones, {} etiquetas)",
        t.as_secs_f64() * 1000.0,
        e.memories.len(),
        e.collections.len(),
        e.tags.len()
    );
    assert_eq!(e.collections.len(), 6);
    assert!(!e.memories.is_empty());
    assert!(t < Duration::from_millis(100), "{t:?}");
}
