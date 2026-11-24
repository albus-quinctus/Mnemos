//! Medición de la comprobación al abrir (spec 022) con 10 000 fotos, 50
//! etiquetas y 20 álbumes.
//!
//! Las fotos son solo filas de la base de datos. No corre con `cargo test`
//! normal. Se lanza en modo release:
//!
//! ```text
//! cargo test --release -p mnemos-core --test rendimiento_apertura -- --ignored --nocapture
//! ```

#![cfg(test)]

use std::time::{Duration, Instant};

use mnemos_core::Library;
use rusqlite::params;

const FOTOS: i64 = 10_000;
/// 2026-11-16 12:00 UTC.
const HOY: i64 = 1_794_830_400;

#[test]
#[ignore]
fn abrir_con_10_000_fotos() {
    let dir = tempfile::tempdir().unwrap();
    {
        let lib = Library::create(dir.path()).unwrap();
        let conn = lib.connection();
        conn.execute_batch("BEGIN").unwrap();
        for i in 1..=FOTOS {
            let taken = HOY - (i * 473_053) % (15 * 365 * 86_400);
            conn.execute(
                "INSERT INTO photos (id, rel_path, size_bytes, modified_at, taken_at, imported_at,
                                     width, height, media_kind, content_hash, latitude,
                                     longitude, favorite)
                 VALUES (?1, ?2, 4000000, ?3, ?3, ?4, 4000, 3000, 'foto', ?5, 40.4,
                         -3.7, ?6)",
                params![
                    i,
                    format!("Originales/{}/IMG_{i:05}.jpg", 2011 + i % 15),
                    taken,
                    HOY,
                    format!("{:064x}", i * 2_654_435_761),
                    i % 13 == 0,
                ],
            )
            .unwrap();
        }
        for t in 1..=50 {
            conn.execute(
                "INSERT INTO tags (id, name) VALUES (?1, ?2)",
                params![t, format!("etiqueta {t}")],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO photo_tags (photo_id, tag_id) SELECT id, ?1 FROM photos WHERE id % 50 = ?1 - 1",
                [t],
            )
            .unwrap();
        }
        for a in 1..=20 {
            conn.execute(
                "INSERT INTO albums (id, name, created_at) VALUES (?1, ?2, ?3)",
                params![a, format!("Álbum {a}"), HOY],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO album_photos (album_id, photo_id, position) SELECT ?1, id, id FROM photos WHERE id % 20 = ?1 - 1",
                [a],
            )
            .unwrap();
        }
        conn.execute_batch("COMMIT").unwrap();
    }
    let db = dir.path().join(".mnemos").join("mnemos.db");
    let mb = std::fs::metadata(&db).unwrap().len() as f64 / 1e6;

    let mut abrir = Duration::MAX;
    let mut comprobar = Duration::MAX;
    for _ in 0..5 {
        let t = Instant::now();
        let lib = Library::open(dir.path()).unwrap();
        abrir = abrir.min(t.elapsed());
        let t = Instant::now();
        let r: String = lib
            .connection()
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .unwrap();
        comprobar = comprobar.min(t.elapsed());
        assert_eq!(r, "ok");
    }
    println!("base: {mb:.1} MB");
    println!("abrir (con quick_check): {abrir:?}");
    println!("quick_check solo: {comprobar:?}");
    assert!(comprobar < Duration::from_millis(200));
}
