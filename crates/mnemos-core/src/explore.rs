//! Lo que enseña la pantalla Explorar (spec 021), en una sola llamada:
//! recuerdos de esta semana en otros años, colecciones automáticas y las
//! etiquetas con más fotos.
//!
//! Solo lee la base de datos: no abre ni toca ningún archivo.

use rusqlite::{OptionalExtension, params_from_iter};
use serde::Serialize;

use crate::collections::Collection;
use crate::error::Result;
use crate::library::Library;
use crate::tags::Tag;

/// Etiquetas como máximo en la fila «Tus etiquetas».
pub const EXPLORE_TAGS: usize = 10;

/// Años hacia atrás que mira Recuerdos como mucho, aunque haya fotos más
/// antiguas (o con una fecha absurda).
const MAX_YEARS: u32 = 150;

/// Las tres filas de Explorar. Una fila vacía viene vacía: la interfaz no la
/// enseña.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Explore {
    /// Del año más reciente al más antiguo.
    pub memories: Vec<Memory>,
    /// En el orden de [`Collection::ALL`], solo las que tienen algo.
    pub collections: Vec<CollectionCard>,
    /// Las [`EXPLORE_TAGS`] etiquetas con más fotos.
    pub tags: Vec<Tag>,
    /// Cuántas etiquetas hay en total, para la tarjeta «Ver todas».
    pub tags_total: u64,
}

/// Una tarjeta de Recuerdos: las fotos de esta semana de hace `years_ago`
/// años. Se abre con `View::Range { desde, hasta }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Memory {
    pub years_ago: u32,
    /// Medianoche local de 3 días antes del mismo día de aquel año (segundos
    /// Unix).
    pub desde: i64,
    /// Medianoche local de 4 días después: el rango es `[desde, hasta)`.
    pub hasta: i64,
    pub photos: u64,
    /// La favorita más reciente de esos días o, si no hay, la más reciente.
    pub cover: i64,
    pub cover_edit_version: Option<u32>,
}

/// Una tarjeta de la fila «Colecciones».
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CollectionCard {
    pub collection: Collection,
    pub photos: u64,
    /// La más reciente de la colección, en su mismo orden.
    pub cover: i64,
    pub cover_edit_version: Option<u32>,
    /// Duración de la portada si es un vídeo, para la etiqueta de la tarjeta.
    pub cover_duration_ms: Option<u64>,
    /// Solo en «Añadidas recientemente»: cuándo fue la última importación.
    pub last_import: Option<i64>,
}

/// Fotos que cuentan para Recuerdos: con fecha de captura, sin archivadas ni
/// Papelera.
const MEMORY_BASE: &str = "taken_at IS NOT NULL AND archived_at IS NULL AND trashed_at IS NULL";

impl Library {
    /// Las filas de Explorar para el instante `hoy` (segundos Unix). Los días
    /// van en la hora local del equipo, como las cabeceras de la cuadrícula.
    pub fn explore(&self, hoy: i64) -> Result<Explore> {
        let mut tags: Vec<Tag> = self.tags()?;
        let tags_total = tags.len() as u64;
        tags.retain(|t| t.photos > 0);
        // Más fotos primero; a igualdad, el orden por nombre de `tags()`.
        tags.sort_by_key(|t| std::cmp::Reverse(t.photos));
        tags.truncate(EXPLORE_TAGS);
        Ok(Explore {
            memories: self.memories(hoy)?,
            collections: self.collection_cards()?,
            tags,
            tags_total,
        })
    }

    fn memories(&self, hoy: i64) -> Result<Vec<Memory>> {
        // Años con fotos hacia atrás, en hora local.
        let oldest: Option<i64> = self.conn.query_row(
            &format!(
                "SELECT CAST(strftime('%Y', ?1, 'unixepoch', 'localtime') AS INTEGER)
                      - CAST(strftime('%Y', min(taken_at), 'unixepoch', 'localtime') AS INTEGER)
                   FROM photos WHERE {MEMORY_BASE}"
            ),
            [hoy],
            |r| r.get(0),
        )?;
        let years = oldest.unwrap_or(0).clamp(0, i64::from(MAX_YEARS)) as u32;

        // El mismo día de hace `n` años, en hora local; SQLite pasa el 29 de
        // febrero al 1 de marzo en los años no bisiestos y la ventana de 7
        // días lo sigue cubriendo.
        let mut range = self.conn.prepare(
            "SELECT unixepoch(?1, 'unixepoch', 'localtime', 'start of day',
                              printf('-%d years', ?2), '-3 days', 'utc'),
                    unixepoch(?1, 'unixepoch', 'localtime', 'start of day',
                              printf('-%d years', ?2), '+4 days', 'utc')",
        )?;
        let mut count = self.conn.prepare(&format!(
            "SELECT count(*) FROM photos
              WHERE {MEMORY_BASE} AND taken_at >= ?1 AND taken_at < ?2"
        ))?;
        let mut cover = self.conn.prepare(&format!(
            "SELECT id, edit_version FROM photos
              WHERE {MEMORY_BASE} AND taken_at >= ?1 AND taken_at < ?2
              ORDER BY favorite DESC, taken_at DESC, id DESC LIMIT 1"
        ))?;

        let mut memories = Vec::new();
        for n in 1..=years {
            let (desde, hasta): (i64, i64) =
                range.query_row((hoy, n), |r| Ok((r.get(0)?, r.get(1)?)))?;
            let photos: u64 = count.query_row((desde, hasta), |r| r.get(0))?;
            if photos == 0 {
                continue;
            }
            let (cover, cover_edit_version) =
                cover.query_row((desde, hasta), |r| Ok((r.get(0)?, r.get(1)?)))?;
            memories.push(Memory {
                years_ago: n,
                desde,
                hasta,
                photos,
                cover,
                cover_edit_version,
            });
        }
        Ok(memories)
    }

    fn collection_cards(&self) -> Result<Vec<CollectionCard>> {
        let mut cards = Vec::new();
        for collection in Collection::ALL {
            let filter = collection.filter();
            let photos: u64 = self.conn.query_row(
                &format!("SELECT count(*) FROM photos WHERE 1 {}", filter.sql),
                params_from_iter(&filter.params),
                |r| r.get(0),
            )?;
            if photos == 0 {
                continue;
            }
            let order = filter.order;
            let row: Option<(i64, Option<u32>, Option<u64>, i64)> = self
                .conn
                .query_row(
                    &format!(
                        "SELECT id, edit_version, duration_ms, {order} FROM photos
                          WHERE 1 {} ORDER BY {order} DESC, id DESC LIMIT 1",
                        filter.sql
                    ),
                    params_from_iter(&filter.params),
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()?;
            let Some((cover, cover_edit_version, cover_duration_ms, date)) = row else {
                continue;
            };
            cards.push(CollectionCard {
                collection,
                photos,
                cover,
                cover_edit_version,
                cover_duration_ms,
                last_import: (collection == Collection::Recent).then_some(date),
            });
        }
        Ok(cards)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::View;
    use rusqlite::params;

    fn biblioteca() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        (dir, lib)
    }

    /// Segundos Unix de una fecha y hora locales, `"2024-12-31 12:00"`.
    fn local(lib: &Library, fecha: &str) -> i64 {
        lib.conn
            .query_row("SELECT unixepoch(?1, 'utc')", [fecha], |r| r.get(0))
            .unwrap()
    }

    #[derive(Default)]
    struct Foto<'a> {
        rel: &'a str,
        taken: Option<i64>,
        imported: i64,
        favorite: bool,
        archived: bool,
        trashed: bool,
        kind: Option<&'a str>,
        duration: Option<u64>,
    }

    fn insertar(lib: &Library, f: Foto) -> i64 {
        lib.conn
            .execute(
                "INSERT INTO photos (rel_path, size_bytes, modified_at, taken_at, imported_at,
                                     favorite, archived_at, trashed_at, media_kind, duration_ms,
                                     width, height)
                 VALUES (?1, 1, 1000, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 4000, 3000)",
                params![
                    f.rel,
                    f.taken,
                    f.imported,
                    f.favorite,
                    f.archived.then_some(1),
                    f.trashed.then_some(1),
                    f.kind.unwrap_or("foto"),
                    f.duration,
                ],
            )
            .unwrap();
        lib.conn.last_insert_rowid()
    }

    fn foto(lib: &Library, rel: &str, fecha: &str) -> i64 {
        let taken = Some(local(lib, fecha));
        insertar(
            lib,
            Foto {
                rel,
                taken,
                ..Foto::default()
            },
        )
    }

    #[test]
    fn biblioteca_vacia_sin_filas() {
        let (_d, lib) = biblioteca();
        let e = lib.explore(local(&lib, "2026-11-16 12:00")).unwrap();
        assert!(e.memories.is_empty() && e.collections.is_empty() && e.tags.is_empty());
        assert_eq!(e.tags_total, 0);
    }

    #[test]
    fn recuerdos_de_esta_semana_en_otros_anos() {
        let (_d, lib) = biblioteca();
        let hoy = local(&lib, "2026-11-16 12:00");
        let a = foto(&lib, "Originales/a.jpg", "2025-11-13 09:00"); // 3 días antes
        let b = foto(&lib, "Originales/b.jpg", "2025-11-19 23:30"); // 3 días después
        foto(&lib, "Originales/fuera.jpg", "2025-11-20 00:10");
        foto(&lib, "Originales/antes.jpg", "2025-11-12 23:50");
        let c = foto(&lib, "Originales/c.jpg", "2019-11-16 10:00");
        // Este año no es un recuerdo.
        foto(&lib, "Originales/hoy.jpg", "2026-11-15 10:00");

        let e = lib.explore(hoy).unwrap();
        let resumen: Vec<_> = e
            .memories
            .iter()
            .map(|m| (m.years_ago, m.photos, m.cover))
            .collect();
        assert_eq!(resumen, [(1, 2, b), (7, 1, c)]);
        let m = &e.memories[0];
        assert_eq!(m.desde, local(&lib, "2025-11-13 00:00"));
        assert_eq!(m.hasta, local(&lib, "2025-11-20 00:00"));

        // La tarjeta abre exactamente sus fotos.
        let vista = View::Range {
            desde: m.desde,
            hasta: m.hasta,
        };
        let ids: Vec<_> = lib
            .view_page(vista, None, 10)
            .unwrap()
            .photos
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(ids, [b, a]);
    }

    #[test]
    fn la_portada_del_recuerdo_es_la_favorita() {
        let (_d, lib) = biblioteca();
        let hoy = local(&lib, "2026-11-16 12:00");
        let fav = insertar(
            &lib,
            Foto {
                rel: "Originales/fav.jpg",
                taken: Some(local(&lib, "2024-11-14 10:00")),
                favorite: true,
                ..Foto::default()
            },
        );
        foto(&lib, "Originales/reciente.jpg", "2024-11-17 10:00");
        let e = lib.explore(hoy).unwrap();
        assert_eq!(e.memories[0].cover, fav);
        assert_eq!(e.memories[0].years_ago, 2);
    }

    #[test]
    fn recuerdos_que_cruzan_el_ano() {
        let (_d, lib) = biblioteca();
        let hoy = local(&lib, "2026-01-02 12:00");
        let nochevieja = foto(&lib, "Originales/nochevieja.jpg", "2024-12-31 23:00");
        let e = lib.explore(hoy).unwrap();
        assert_eq!(e.memories.len(), 1);
        let m = &e.memories[0];
        assert_eq!((m.years_ago, m.cover), (1, nochevieja));
        assert_eq!(m.desde, local(&lib, "2024-12-30 00:00"));
        assert_eq!(m.hasta, local(&lib, "2025-01-06 00:00"));
    }

    #[test]
    fn veintinueve_de_febrero() {
        let (_d, lib) = biblioteca();
        let bisiesto = foto(&lib, "Originales/29.jpg", "2024-02-29 12:00");
        let dia28 = foto(&lib, "Originales/28.jpg", "2027-02-28 12:00");

        // Desde un 29 de febrero, el año no bisiesto cae en el 1 de marzo y
        // la ventana sigue cogiendo el 28.
        let e = lib.explore(local(&lib, "2028-02-29 12:00")).unwrap();
        let r: Vec<_> = e.memories.iter().map(|m| (m.years_ago, m.cover)).collect();
        assert_eq!(r, [(1, dia28), (4, bisiesto)]);

        // Desde un año no bisiesto, el 29 de febrero entra por la ventana.
        let e = lib.explore(local(&lib, "2027-03-01 12:00")).unwrap();
        let r: Vec<_> = e.memories.iter().map(|m| (m.years_ago, m.cover)).collect();
        assert_eq!(r, [(3, bisiesto)]);
    }

    #[test]
    fn recuerdos_sin_archivadas_papelera_ni_fotos_sin_fecha() {
        let (_d, lib) = biblioteca();
        let hoy = local(&lib, "2026-11-16 12:00");
        let hace_un_ano = Some(local(&lib, "2025-11-16 12:00"));
        for (rel, archived, trashed) in [("arch.jpg", true, false), ("pap.jpg", false, true)] {
            insertar(
                &lib,
                Foto {
                    rel,
                    taken: hace_un_ano,
                    archived,
                    trashed,
                    ..Foto::default()
                },
            );
        }
        // Sin fecha de captura: la de modificación no cuenta.
        insertar(
            &lib,
            Foto {
                rel: "sin-fecha.jpg",
                ..Foto::default()
            },
        );
        assert!(lib.explore(hoy).unwrap().memories.is_empty());
    }

    #[test]
    fn colecciones_con_numero_y_portada() {
        let (_d, lib) = biblioteca();
        let viejo = insertar(
            &lib,
            Foto {
                rel: "Originales/viejo.mp4",
                taken: Some(100),
                imported: 50,
                kind: Some("video"),
                duration: Some(42_000),
                ..Foto::default()
            },
        );
        let nuevo = insertar(
            &lib,
            Foto {
                rel: "Originales/nuevo.mp4",
                taken: Some(200),
                imported: 10,
                kind: Some("video"),
                duration: Some(5_000),
                ..Foto::default()
            },
        );
        insertar(
            &lib,
            Foto {
                rel: "Originales/archivado.mp4",
                taken: Some(300),
                imported: 99,
                kind: Some("video"),
                archived: true,
                ..Foto::default()
            },
        );
        let e = lib.explore(1_000_000).unwrap();
        let tipos: Vec<_> = e.collections.iter().map(|c| c.collection).collect();
        // Sin capturas, panorámicas, editadas ni RAW: esas tarjetas no salen.
        assert_eq!(tipos, [Collection::Videos, Collection::Recent]);

        let videos = &e.collections[0];
        assert_eq!((videos.photos, videos.cover), (2, nuevo));
        assert_eq!(videos.cover_duration_ms, Some(5_000));
        assert_eq!(videos.last_import, None);

        // Añadidas recientemente va por la fecha de importación.
        let recientes = &e.collections[1];
        assert_eq!((recientes.photos, recientes.cover), (2, viejo));
        assert_eq!(recientes.last_import, Some(50));
    }

    #[test]
    fn las_etiquetas_con_mas_fotos() {
        let (_d, mut lib) = biblioteca();
        let ids: Vec<i64> = (0..12)
            .map(|i| foto(&lib, &format!("Originales/{i}.jpg"), "2020-01-01 12:00"))
            .collect();
        // Etiqueta i con i + 1 fotos, y una vacía.
        for i in 0..12 {
            let t = lib.create_tag(&format!("t{i:02}")).unwrap();
            lib.add_tag(&ids[..=i], t.id).unwrap();
        }
        lib.create_tag("vacía").unwrap();

        let e = lib.explore(local(&lib, "2026-11-16 12:00")).unwrap();
        let nombres: Vec<_> = e.tags.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            nombres,
            [
                "t11", "t10", "t09", "t08", "t07", "t06", "t05", "t04", "t03", "t02"
            ]
        );
        assert_eq!(e.tags_total, 13);
    }
}
