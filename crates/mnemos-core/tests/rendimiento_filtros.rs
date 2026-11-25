//! Medición de los filtros de la búsqueda (spec 024) con 10 000 fotos.
//!
//! Las fotos son solo filas de la base de datos: los filtros no abren ningún
//! archivo. No corre con `cargo test` normal. Se lanza en modo release:
//!
//! ```text
//! cargo test --release -p mnemos-core --test rendimiento_filtros -- --ignored --nocapture
//! ```

#![cfg(test)]

use std::time::{Duration, Instant};

use mnemos_core::{KindFilter, Library, SearchFilters, SearchQuery};
use rusqlite::params;

const FOTOS: i64 = 10_000;
/// 2026-10-26 a mediodía, en hora de pared.
const AHORA: i64 = 1_793_016_000;
const QUINCE_ANOS: i64 = 15 * 365 * 86_400;

const CAMARAS: [(&str, &[&str]); 5] = [
    (
        "Canon EOS 80D",
        &["EF-S 18-55mm", "EF 50mm f/1.8", "EF 70-200mm"],
    ),
    ("Pixel 7", &["Pixel 7 back camera", "Pixel 7 front camera"]),
    ("iPhone 13", &["iPhone 13 back dual wide camera"]),
    ("Fujifilm X-T3", &["XF 23mm F2", "XF 16-80mm"]),
    ("DMC-TZ7", &[]),
];

fn medir<T>(nombre: &str, f: impl FnOnce() -> T) -> (T, Duration) {
    let inicio = Instant::now();
    let r = f();
    let t = inicio.elapsed();
    println!("{nombre:<50} {:>8.1} ms", t.as_secs_f64() * 1000.0);
    (r, t)
}

#[test]
#[ignore]
fn filtros_con_10_000_fotos() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = Library::create(dir.path()).unwrap();
    {
        let conn = lib.connection();
        conn.execute_batch("BEGIN").unwrap();
        for i in 1..=FOTOS {
            // Una de cada 10 sin cámara (capturas, descargas); vídeos y RAW
            // repartidos; una de cada 4 con estrellas.
            let (camara, objetivos) = CAMARAS[(i % 5) as usize];
            let camara = (i % 10 != 1).then_some(camara);
            let objetivo = camara
                .and_then(|_| objetivos.get((i / 5) as usize % objetivos.len().max(1)))
                .copied();
            let (ext, kind) = match i % 20 {
                19 => ("mp4", "video"),
                1 | 2 => ("cr2", "foto"),
                _ => ("jpg", "foto"),
            };
            let rel = format!("Originales/{:03}/IMG_{i:05}.{ext}", i / 1000);
            let fecha = AHORA - (i * 47_303) % QUINCE_ANOS;
            let rating = if i % 4 == 0 { i % 5 + 1 } else { 0 };
            conn.execute(
                "INSERT INTO photos (id, rel_path, size_bytes, modified_at, imported_at,
                                     taken_local, media_kind, camera, lens, rating,
                                     favorite, archived_at)
                 VALUES (?1, ?2, 1, ?3, 0, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    i,
                    rel,
                    fecha,
                    kind,
                    camara,
                    objetivo,
                    rating,
                    i % 7 == 3,
                    (i % 9 == 0).then_some(1),
                ],
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
    // 30 álbumes de 300 fotos y 100 etiquetas de 200.
    for a in 0..30 {
        let album = lib.create_album(&format!("Álbum {a:02}")).unwrap();
        let ids: Vec<i64> = (0..300).map(|k| (a * 31 + k * 33) % FOTOS + 1).collect();
        lib.add_to_album(album, &ids).unwrap();
    }
    let mut etiqueta = 0;
    for t in 0..100 {
        let tag = lib.create_tag(&format!("Etiqueta {t:03}")).unwrap();
        let ids: Vec<i64> = (0..200).map(|k| (t * 37 + k * 50) % FOTOS + 1).collect();
        lib.add_tag(&ids, tag.id).unwrap();
        etiqueta = tag.id;
    }
    let album = lib.albums().unwrap()[0].id;

    let (opciones, t_opciones) = medir("opciones del panel (filter_options)", || {
        lib.filter_options().unwrap()
    });
    assert_eq!(opciones.cameras.len(), CAMARAS.len());
    assert_eq!(opciones.albums.len(), 30);
    assert_eq!(opciones.tags.len(), 100);

    let nada = SearchFilters::default();
    let casos: Vec<(&str, &str, SearchFilters)> = vec![
        ("sin filtros ni texto (todas)", "", nada.clone()),
        (
            "tipo vídeos",
            "",
            SearchFilters {
                kind: Some(KindFilter::Videos),
                ..nada.clone()
            },
        ),
        (
            "tipo RAW",
            "",
            SearchFilters {
                kind: Some(KindFilter::Raw),
                ..nada.clone()
            },
        ),
        (
            "cámara",
            "",
            SearchFilters {
                camera: Some("Canon EOS 80D".into()),
                ..nada.clone()
            },
        ),
        (
            "cámara y objetivo",
            "",
            SearchFilters {
                camera: Some("Canon EOS 80D".into()),
                lens: Some("EF 50mm f/1.8".into()),
                ..nada.clone()
            },
        ),
        (
            "rango de un año",
            "",
            SearchFilters {
                from: Some("2020-01-01".into()),
                to: Some("2020-12-31".into()),
                ..nada.clone()
            },
        ),
        (
            "★ o más",
            "",
            SearchFilters {
                min_rating: Some(1),
                ..nada.clone()
            },
        ),
        (
            "★★★★★",
            "",
            SearchFilters {
                min_rating: Some(5),
                ..nada.clone()
            },
        ),
        (
            "solo favoritas",
            "",
            SearchFilters {
                favorites: true,
                ..nada.clone()
            },
        ),
        (
            "álbum",
            "",
            SearchFilters {
                album: Some(album),
                ..nada.clone()
            },
        ),
        (
            "etiqueta",
            "",
            SearchFilters {
                tag: Some(etiqueta),
                ..nada.clone()
            },
        ),
        (
            "texto «img» y fotos de 3★ o más",
            "img",
            SearchFilters {
                kind: Some(KindFilter::Photos),
                min_rating: Some(3),
                ..nada.clone()
            },
        ),
        (
            "texto «2024» y cámara",
            "2024",
            SearchFilters {
                camera: Some("Pixel 7".into()),
                ..nada.clone()
            },
        ),
        (
            "todo a la vez",
            "img",
            SearchFilters {
                kind: Some(KindFilter::Photos),
                camera: Some("Canon EOS 80D".into()),
                lens: Some("EF-S 18-55mm".into()),
                from: Some("2015-01-01".into()),
                to: Some("2026-12-31".into()),
                min_rating: Some(1),
                favorites: true,
                album: None,
                tag: None,
            },
        ),
    ];
    let mut peor = Duration::ZERO;
    for (nombre, texto, filters) in casos {
        let query = SearchQuery {
            filters,
            ..SearchQuery::parse(texto)
        };
        let ((meses, pagina), t) = medir(&format!("meses y 1.ª página: {nombre}"), || {
            (
                lib.search_months(&query).unwrap(),
                lib.search_page(&query, None, 500).unwrap(),
            )
        });
        let total: u64 = meses.iter().map(|m| m.photos).sum();
        println!("{:<50} {total:>8}", "  fotos encontradas");
        assert!(pagina.photos.len() as u64 <= total);
        peor = peor.max(t);
    }

    assert!(
        t_opciones < Duration::from_millis(100),
        "opciones: {t_opciones:?}"
    );
    assert!(peor < Duration::from_millis(100), "peor búsqueda: {peor:?}");
}
