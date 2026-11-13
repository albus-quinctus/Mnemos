//! Medición de las etiquetas (spec 016) con 10 000 fotos y 500 etiquetas.
//!
//! Las fotos son solo filas de la base de datos: las etiquetas no abren
//! ningún archivo. No corre con `cargo test` normal. Se lanza en modo release:
//!
//! ```text
//! cargo test --release -p mnemos-core --test rendimiento_etiquetas -- --ignored --nocapture
//! ```

#![cfg(test)]

use std::time::{Duration, Instant};

use mnemos_core::{Library, SearchQuery};
use rusqlite::params;

const FOTOS: i64 = 10_000;
const ETIQUETAS: i64 = 500;

fn medir<T>(nombre: &str, f: impl FnOnce() -> T) -> (T, Duration) {
    let inicio = Instant::now();
    let r = f();
    let t = inicio.elapsed();
    println!("{nombre:<45} {:>8.1} ms", t.as_secs_f64() * 1000.0);
    (r, t)
}

#[test]
#[ignore]
fn etiquetas_con_10_000_fotos() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = Library::create(dir.path()).unwrap();
    {
        let conn = lib.connection();
        conn.execute_batch("BEGIN").unwrap();
        for i in 1..=FOTOS {
            let rel = format!("Originales/{:03}/IMG_{i:05}.jpg", i / 1000);
            conn.execute(
                "INSERT INTO photos (id, rel_path, size_bytes, modified_at, imported_at)
                 VALUES (?1, ?2, 1, ?3, 0)",
                params![i, rel, 1_700_000_000 - (i * 7919) % 94_608_000],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO photo_search (rowid, rel_path, tags) VALUES (?1, ?2, '')",
                params![i, rel],
            )
            .unwrap();
        }
        conn.execute_batch("COMMIT").unwrap();
    }
    let todas: Vec<i64> = (1..=FOTOS).collect();

    let playa = lib.create_tag("Playa").unwrap();
    let (puestas, t_todas) = medir("poner una etiqueta a las 10 000", || {
        lib.add_tag(&todas, playa.id).unwrap()
    });
    assert_eq!(puestas.len(), FOTOS as usize);

    // 499 etiquetas más, cada una en unas 200 fotos repartidas.
    for t in 1..ETIQUETAS {
        let tag = lib.create_tag(&format!("Etiqueta {t:03}")).unwrap();
        let ids: Vec<i64> = (0..200).map(|k| (t * 37 + k * 50) % FOTOS + 1).collect();
        lib.add_tag(&ids, tag.id).unwrap();
    }

    let (lista, t_lista) = medir("listar 500 etiquetas con recuento y portada", || {
        lib.tags().unwrap()
    });
    assert_eq!(lista.len(), ETIQUETAS as usize);
    let (uso, t_uso) = medir("uso de las etiquetas en 10 000 seleccionadas", || {
        lib.tag_usage(&todas).unwrap()
    });
    assert_eq!(uso.len(), ETIQUETAS as usize);
    let (_, t_buscar) = medir("buscar «playa» (las 10 000)", || {
        let q = SearchQuery::parse("playa");
        (
            lib.search_months(&q).unwrap(),
            lib.search_page(&q, None, 500).unwrap(),
        )
    });
    let (_, t_renombrar) = medir("renombrar «Playa» (10 000 fotos)", || {
        lib.rename_tag(playa.id, "Costa").unwrap()
    });
    let (_, t_quitar) = medir("quitar la etiqueta a las 10 000", || {
        lib.remove_tag(&todas, playa.id).unwrap()
    });

    assert!(t_todas < Duration::from_millis(300), "poner: {t_todas:?}");
    assert!(
        t_quitar < Duration::from_millis(300),
        "quitar: {t_quitar:?}"
    );
    assert!(t_lista < Duration::from_millis(50), "listar: {t_lista:?}");
    assert!(t_uso < Duration::from_millis(50), "uso: {t_uso:?}");
    assert!(
        t_buscar < Duration::from_millis(100),
        "buscar: {t_buscar:?}"
    );
    assert!(
        t_renombrar < Duration::from_millis(300),
        "renombrar: {t_renombrar:?}"
    );
}
