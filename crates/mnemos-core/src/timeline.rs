//! Fotos en orden cronológico para la cuadrícula (spec 002).
//!
//! La interfaz nunca pide todas las fotos: primero los meses con su número de
//! fotos, para reservar el alto de la cuadrícula, y después las fotos por
//! páginas. La paginación va por cursor (fecha e id de la última foto), así que
//! cada página cuesta lo mismo aunque la biblioteca tenga 100 000 fotos.

use rusqlite::params_from_iter;
use rusqlite::types::Value;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::library::Library;

/// Fotos por página como máximo, pida lo que pida la interfaz.
pub const MAX_PAGE: u32 = 1000;

/// Fecha por la que se ordena: la de captura o, si falta, la de modificación.
pub(crate) const DATE: &str = "coalesce(taken_at, modified_at)";

/// Condición extra sobre `photos` para la búsqueda: un trozo de SQL que
/// empieza por `AND`, con parámetros anónimos (`?`), y sus valores.
#[derive(Debug, Default)]
pub(crate) struct Filter {
    pub sql: String,
    pub params: Vec<Value>,
}

/// Un mes con fotos, en hora local: `"2026-10"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Month {
    pub month: String,
    pub photos: u64,
}

/// Lo que la cuadrícula necesita de cada foto. Las dimensiones ya tienen en
/// cuenta la orientación; faltan si no se pudieron leer de la cabecera.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GridPhoto {
    pub id: i64,
    /// Nombre del archivo, sin carpetas, para el marcador "Sin vista previa".
    pub name: String,
    pub month: String,
    pub date: i64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// La miniatura no se pudo generar: la cuadrícula muestra el marcador
    /// "Sin vista previa" en vez de esperarla.
    pub thumbnail_failed: bool,
}

/// Dónde sigue la página siguiente: justo después de esta foto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineCursor {
    pub date: i64,
    pub id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TimelinePage {
    pub photos: Vec<GridPhoto>,
    /// `None` cuando ya no quedan fotos.
    pub next: Option<TimelineCursor>,
}

impl Library {
    /// Meses con fotos, del más reciente al más antiguo.
    pub fn timeline_months(&self) -> Result<Vec<Month>> {
        self.months_where(&Filter::default())
    }

    /// Hasta `limit` fotos a partir de `after` (o desde la más reciente).
    pub fn timeline_page(&self, after: Option<TimelineCursor>, limit: u32) -> Result<TimelinePage> {
        self.page_where(&Filter::default(), after, limit)
    }

    pub(crate) fn months_where(&self, filter: &Filter) -> Result<Vec<Month>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT strftime('%Y-%m', {DATE}, 'unixepoch', 'localtime') AS month, count(*)
             FROM photos WHERE 1 {} GROUP BY month ORDER BY month DESC",
            filter.sql
        ))?;
        let months = stmt
            .query_map(params_from_iter(&filter.params), |r| {
                Ok(Month {
                    month: r.get(0)?,
                    photos: r.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(months)
    }

    pub(crate) fn page_where(
        &self,
        filter: &Filter,
        after: Option<TimelineCursor>,
        limit: u32,
    ) -> Result<TimelinePage> {
        let limit = limit.clamp(1, MAX_PAGE);
        let (date, id) = after.map_or((i64::MAX, i64::MAX), |c| (c.date, c.id));
        // Se pide una de más para saber si hay página siguiente.
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, {DATE}, strftime('%Y-%m', {DATE}, 'unixepoch', 'localtime'),
                    width, height, coalesce(orientation, 1),
                    content_hash IN (SELECT content_hash FROM thumbnail_failures),
                    rel_path
             FROM photos
             WHERE ({DATE}, id) < (?, ?) {}
             ORDER BY {DATE} DESC, id DESC
             LIMIT ?",
            filter.sql
        ))?;
        let params = [Value::from(date), Value::from(id)]
            .into_iter()
            .chain(filter.params.iter().cloned())
            .chain([Value::from(i64::from(limit) + 1)]);
        let mut photos = stmt
            .query_map(params_from_iter(params), |r| {
                let (w, h): (Option<u32>, Option<u32>) = (r.get(3)?, r.get(4)?);
                let orientation: u16 = r.get(5)?;
                let (width, height) = if mnemos_imaging::swaps_dimensions(orientation) {
                    (h, w)
                } else {
                    (w, h)
                };
                let rel_path: String = r.get(7)?;
                Ok(GridPhoto {
                    id: r.get(0)?,
                    name: rel_path.rsplit('/').next().unwrap_or_default().to_owned(),
                    date: r.get(1)?,
                    month: r.get(2)?,
                    width,
                    height,
                    thumbnail_failed: r.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let next = if photos.len() > limit as usize {
            photos.truncate(limit as usize);
            photos.last().map(|p| TimelineCursor {
                date: p.date,
                id: p.id,
            })
        } else {
            None
        };
        Ok(TimelinePage { photos, next })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    // Mediados de mes, para que la zona horaria del equipo no cambie el mes.
    const OCT_2026: i64 = 1_791_892_800; // 2026-10-13 12:00 UTC
    const SEP_2026: i64 = 1_789_214_400; // 2026-09-12 12:00 UTC
    const MAY_2024: i64 = 1_715_774_400; // 2024-05-15 12:00 UTC

    fn library(rows: &[(&str, i64, Option<i64>, u16)]) -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        for (rel, modified, taken, orientation) in rows {
            lib.conn
                .execute(
                    "INSERT INTO photos (rel_path, size_bytes, modified_at, taken_at, width,
                                         height, orientation, imported_at)
                     VALUES (?1, 1, ?2, ?3, 4000, 3000, ?4, 0)",
                    params![rel, modified, taken, orientation],
                )
                .unwrap();
        }
        (dir, lib)
    }

    fn rel_paths(lib: &Library, page: &TimelinePage) -> Vec<String> {
        page.photos
            .iter()
            .map(|p| {
                lib.photo_media(p.id)
                    .unwrap()
                    .unwrap()
                    .rel_path
                    .as_str()
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn agrupa_por_mes_de_lo_mas_reciente_a_lo_mas_antiguo() {
        let (_dir, lib) = library(&[
            ("Originales/a.jpg", MAY_2024, None, 1),
            ("Originales/b.jpg", OCT_2026, None, 1),
            ("Originales/c.jpg", OCT_2026 + 60, None, 1),
            ("Originales/d.jpg", SEP_2026, None, 1),
        ]);
        let months = lib.timeline_months().unwrap();
        let months: Vec<_> = months
            .iter()
            .map(|m| (m.month.as_str(), m.photos))
            .collect();
        assert_eq!(months, [("2026-10", 2), ("2026-09", 1), ("2024-05", 1)]);

        let page = lib.timeline_page(None, 10).unwrap();
        assert_eq!(
            rel_paths(&lib, &page),
            [
                "Originales/c.jpg",
                "Originales/b.jpg",
                "Originales/d.jpg",
                "Originales/a.jpg"
            ]
        );
        assert_eq!(page.photos[0].month, "2026-10");
        assert_eq!(page.photos[0].name, "c.jpg");
        assert_eq!(page.next, None);
    }

    #[test]
    fn la_fecha_de_captura_manda_sobre_la_de_modificacion() {
        let (_dir, lib) = library(&[
            ("Originales/copiada-hoy.jpg", OCT_2026, Some(MAY_2024), 1),
            ("Originales/b.jpg", SEP_2026, None, 1),
        ]);
        let page = lib.timeline_page(None, 10).unwrap();
        assert_eq!(
            rel_paths(&lib, &page),
            ["Originales/b.jpg", "Originales/copiada-hoy.jpg"]
        );
        assert_eq!(page.photos[1].month, "2024-05");
    }

    #[test]
    fn pagina_sin_repetir_ni_saltar_fotos_con_la_misma_fecha() {
        let rows: Vec<_> = (0..25)
            .map(|i| (format!("Originales/{i:02}.jpg"), OCT_2026 + i / 10))
            .collect();
        let rows: Vec<_> = rows
            .iter()
            .map(|(r, d)| (r.as_str(), *d, None, 1))
            .collect();
        let (_dir, lib) = library(&rows);

        let mut seen = Vec::new();
        let mut after = None;
        let mut pages = 0;
        loop {
            let page = lib.timeline_page(after, 7).unwrap();
            assert!(page.photos.len() <= 7);
            seen.extend(page.photos.iter().map(|p| p.id));
            pages += 1;
            match page.next {
                Some(next) => after = Some(next),
                None => break,
            }
        }
        assert_eq!(pages, 4);
        let mut unique = seen.clone();
        unique.sort();
        unique.dedup();
        assert_eq!((seen.len(), unique.len()), (25, 25));
    }

    #[test]
    fn la_ultima_pagina_exacta_no_deja_una_pagina_vacia() {
        let (_dir, lib) = library(&[
            ("Originales/a.jpg", OCT_2026, None, 1),
            ("Originales/b.jpg", SEP_2026, None, 1),
        ]);
        assert_eq!(lib.timeline_page(None, 2).unwrap().next, None);
    }

    #[test]
    fn las_dimensiones_tienen_en_cuenta_la_orientacion() {
        let (_dir, lib) = library(&[
            ("Originales/movil.jpg", OCT_2026, None, 6),
            ("Originales/camara.jpg", SEP_2026, None, 1),
        ]);
        let page = lib.timeline_page(None, 10).unwrap();
        let dims: Vec<_> = page.photos.iter().map(|p| (p.width, p.height)).collect();
        assert_eq!(dims, [(Some(3000), Some(4000)), (Some(4000), Some(3000))]);
    }

    #[test]
    fn marca_las_fotos_sin_miniatura_posible() {
        let (_dir, lib) = library(&[
            ("Originales/rota.jpg", OCT_2026, None, 1),
            ("Originales/buena.jpg", SEP_2026, None, 1),
        ]);
        lib.conn
            .execute_batch(
                "UPDATE photos SET content_hash = rel_path;
                 INSERT INTO thumbnail_failures VALUES ('Originales/rota.jpg', 'dañada', 0);",
            )
            .unwrap();
        let page = lib.timeline_page(None, 10).unwrap();
        let failed: Vec<_> = page.photos.iter().map(|p| p.thumbnail_failed).collect();
        assert_eq!(failed, [true, false]);
    }

    #[test]
    fn biblioteca_vacia() {
        let (_dir, lib) = library(&[]);
        assert!(lib.timeline_months().unwrap().is_empty());
        let page = lib.timeline_page(None, 0).unwrap();
        assert!(page.photos.is_empty());
        assert_eq!(page.next, None);
    }

    #[test]
    fn la_consulta_usa_el_indice() {
        let (_dir, lib) = library(&[]);
        let plan: Vec<String> = lib
            .conn
            .prepare(&format!(
                "EXPLAIN QUERY PLAN SELECT id FROM photos WHERE ({DATE}, id) < (1, 1)
                 ORDER BY {DATE} DESC, id DESC LIMIT 10"
            ))
            .unwrap()
            .query_map([], |r| r.get(3))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let plan = plan.join(" ");
        assert!(plan.contains("photos_timeline"), "{plan}");
        assert!(!plan.contains("TEMP B-TREE"), "{plan}");
    }
}
