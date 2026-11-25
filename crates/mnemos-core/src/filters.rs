//! Filtros de la búsqueda (spec 024): tipo, cámara, objetivo, fechas,
//! valoración mínima, favoritas, álbum y etiqueta.
//!
//! Cada filtro es una condición más del [`Filter`] de la búsqueda, con sus
//! valores como parámetros: ningún texto de la interfaz se pega en el SQL.

use rusqlite::OptionalExtension;
use rusqlite::types::Value;
use serde::{Deserialize, Serialize};

use crate::collections::raw_condition;
use crate::error::{Error, Result};
use crate::library::Library;
use crate::timeline::{DATE, Filter};

/// Tipo de archivo del filtro «Tipo».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KindFilter {
    /// Fotos, RAW incluidos.
    #[serde(rename = "fotos")]
    Photos,
    #[serde(rename = "videos")]
    Videos,
    /// Solo RAW, con la misma lista de extensiones que la colección RAW de
    /// Explorar (spec 021).
    #[serde(rename = "raw")]
    Raw,
}

/// Filtros del panel de la búsqueda. Todos deben cumplirse, y además el
/// texto. `Default` es «sin filtros».
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchFilters {
    #[serde(rename = "tipo")]
    pub kind: Option<KindFilter>,
    /// Cámara exacta, como sale en [`FilterOptions::cameras`].
    #[serde(rename = "camara")]
    pub camera: Option<String>,
    /// Objetivo exacto, como sale en [`FilterOptions::lenses`].
    #[serde(rename = "objetivo")]
    pub lens: Option<String>,
    /// Primer día, `AAAA-MM-DD`, incluido.
    #[serde(rename = "desde")]
    pub from: Option<String>,
    /// Último día, `AAAA-MM-DD`, incluido.
    #[serde(rename = "hasta")]
    pub to: Option<String>,
    /// De 1 a 5; 0 o nada es «Cualquiera».
    #[serde(rename = "valoracion_minima")]
    pub min_rating: Option<u8>,
    #[serde(rename = "favoritas")]
    pub favorites: bool,
    pub album: Option<i64>,
    #[serde(rename = "etiqueta")]
    pub tag: Option<i64>,
}

impl SearchFilters {
    pub fn is_empty(&self) -> bool {
        self.kind.is_none()
            && self.camera.is_none()
            && self.lens.is_none()
            && self.from.is_none()
            && self.to.is_none()
            && self.min_rating.unwrap_or(0) == 0
            && !self.favorites
            && self.album.is_none()
            && self.tag.is_none()
    }
}

/// Una cámara de la biblioteca y cuántas fotos tiene.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraOption {
    #[serde(rename = "nombre")]
    pub name: String,
    #[serde(rename = "fotos")]
    pub photos: u64,
}

/// Un objetivo usado con una cámara y cuántas fotos tiene esa pareja. La
/// interfaz suma las de todas las cámaras cuando no hay cámara elegida.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LensOption {
    #[serde(rename = "camara")]
    pub camera: Option<String>,
    #[serde(rename = "nombre")]
    pub name: String,
    #[serde(rename = "fotos")]
    pub photos: u64,
}

/// Un álbum o una etiqueta y cuántas fotos tiene.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupOption {
    pub id: i64,
    #[serde(rename = "nombre")]
    pub name: String,
    #[serde(rename = "fotos")]
    pub photos: u64,
}

/// Lo que llena el panel de filtros. Los recuentos no cuentan la Papelera,
/// como la búsqueda.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterOptions {
    /// De la que más fotos tiene a la que menos.
    #[serde(rename = "camaras")]
    pub cameras: Vec<CameraOption>,
    /// De más a menos fotos.
    #[serde(rename = "objetivos")]
    pub lenses: Vec<LensOption>,
    /// Por nombre.
    #[serde(rename = "albumes")]
    pub albums: Vec<GroupOption>,
    /// Por nombre.
    #[serde(rename = "etiquetas")]
    pub tags: Vec<GroupOption>,
}

impl Library {
    /// Añade a `filter` las condiciones de `filters`.
    pub(crate) fn push_filters(&self, filter: &mut Filter, filters: &SearchFilters) -> Result<()> {
        let mut and = |sql: &str, params: Vec<Value>| {
            filter.sql.push_str(" AND ");
            filter.sql.push_str(sql);
            filter.params.extend(params);
        };
        match filters.kind {
            Some(KindFilter::Photos) => and("media_kind = 'foto'", vec![]),
            Some(KindFilter::Videos) => and("media_kind = 'video'", vec![]),
            Some(KindFilter::Raw) => and(&raw_condition(), vec![]),
            None => {}
        }
        if let Some(camera) = &filters.camera {
            and("camera = ?", vec![Value::from(camera.clone())]);
        }
        if let Some(lens) = &filters.lens {
            and("lens = ?", vec![Value::from(lens.clone())]);
        }
        if let Some(from) = &filters.from {
            let from = self.day_start(from, false)?;
            and(&format!("{DATE} >= ?"), vec![Value::from(from)]);
        }
        if let Some(to) = &filters.to {
            let end = self.day_start(to, true)?;
            and(&format!("{DATE} < ?"), vec![Value::from(end)]);
        }
        match filters.min_rating {
            None | Some(0) => {}
            Some(n @ 1..=5) => and("rating >= ?", vec![Value::from(n)]),
            Some(n) => return Err(Error::InvalidRating(n)),
        }
        if filters.favorites {
            and("favorite = 1", vec![]);
        }
        if let Some(album) = filters.album {
            and(
                "id IN (SELECT photo_id FROM album_photos WHERE album_id = ?)",
                vec![Value::from(album)],
            );
        }
        if let Some(tag) = filters.tag {
            and(
                "id IN (SELECT photo_id FROM photo_tags WHERE tag_id = ?)",
                vec![Value::from(tag)],
            );
        }
        Ok(())
    }

    /// Inicio de `day` (o del día siguiente, con `next`) en hora de pared,
    /// como [`DATE`] (spec 022). Error si no es un `AAAA-MM-DD` válido.
    fn day_start(&self, day: &str, next: bool) -> Result<i64> {
        let span = if next { "+1 day" } else { "+0 day" };
        self.conn
            .query_row(
                "SELECT unixepoch(?1, ?2) WHERE date(?1) IS ?1",
                (day, span),
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| Error::InvalidDay(day.to_owned()))
    }

    /// Cámaras, objetivos, álbumes y etiquetas para el panel de filtros.
    pub fn filter_options(&self) -> Result<FilterOptions> {
        let cameras = self
            .conn
            .prepare(
                "SELECT camera, count(*) AS n FROM photos
                 WHERE camera IS NOT NULL AND trashed_at IS NULL
                 GROUP BY camera ORDER BY n DESC, camera",
            )?
            .query_map([], |r| {
                Ok(CameraOption {
                    name: r.get(0)?,
                    photos: r.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let lenses = self
            .conn
            .prepare(
                "SELECT camera, lens, count(*) AS n FROM photos
                 WHERE lens IS NOT NULL AND trashed_at IS NULL
                 GROUP BY camera, lens ORDER BY n DESC, lens, camera",
            )?
            .query_map([], |r| {
                Ok(LensOption {
                    camera: r.get(0)?,
                    name: r.get(1)?,
                    photos: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let albums = self.group_options(
            "SELECT a.id, a.name, count(p.id) FROM albums a
             LEFT JOIN album_photos ap ON ap.album_id = a.id
             LEFT JOIN photos p ON p.id = ap.photo_id AND p.trashed_at IS NULL
             GROUP BY a.id ORDER BY a.name COLLATE NOCASE, a.id",
        )?;
        let tags = self.group_options(
            "SELECT t.id, t.name, count(p.id) FROM tags t
             LEFT JOIN photo_tags pt ON pt.tag_id = t.id
             LEFT JOIN photos p ON p.id = pt.photo_id AND p.trashed_at IS NULL
             GROUP BY t.id ORDER BY t.name COLLATE NOCASE, t.id",
        )?;
        Ok(FilterOptions {
            cameras,
            lenses,
            albums,
            tags,
        })
    }

    fn group_options(&self, sql: &str) -> Result<Vec<GroupOption>> {
        let options = self
            .conn
            .prepare(sql)?
            .query_map([], |r| {
                Ok(GroupOption {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    photos: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use super::*;
    use crate::search::SearchQuery;

    /// 2024-07-14 12:00 en hora de pared.
    const JULIO_14: i64 = 1_720_958_400;
    const DIA: i64 = 86_400;

    struct Foto<'a> {
        rel: &'a str,
        kind: &'a str,
        local: i64,
        camera: Option<&'a str>,
        lens: Option<&'a str>,
        rating: u8,
        favorite: bool,
        trashed: bool,
    }

    const BASE: Foto<'static> = Foto {
        rel: "",
        kind: "foto",
        local: JULIO_14,
        camera: None,
        lens: None,
        rating: 0,
        favorite: false,
        trashed: false,
    };

    fn biblioteca(fotos: &[Foto]) -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        for (i, f) in fotos.iter().enumerate() {
            let id = i as i64 + 1;
            lib.conn
                .execute(
                    "INSERT INTO photos (id, rel_path, size_bytes, modified_at, imported_at,
                                         taken_local, media_kind, camera, lens, rating,
                                         favorite, trashed_at)
                     VALUES (?1, ?2, 1, 0, 0, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        id,
                        f.rel,
                        f.local,
                        f.kind,
                        f.camera,
                        f.lens,
                        f.rating,
                        f.favorite,
                        f.trashed.then_some(1),
                    ],
                )
                .unwrap();
            lib.conn
                .execute(
                    "INSERT INTO photo_search (rowid, rel_path, tags) VALUES (?1, ?2, '')",
                    params![id, f.rel],
                )
                .unwrap();
        }
        (dir, lib)
    }

    /// Nombres de los resultados de `texto` con `filters`, del más reciente al
    /// más antiguo, comprobando que los meses cuentan lo mismo.
    fn buscar(lib: &Library, texto: &str, filters: SearchFilters) -> Vec<String> {
        let query = SearchQuery {
            filters,
            ..SearchQuery::parse(texto)
        };
        let page = lib.search_page(&query, None, 100).unwrap();
        let months: u64 = lib
            .search_months(&query)
            .unwrap()
            .iter()
            .map(|m| m.photos)
            .sum();
        assert_eq!(months, page.photos.len() as u64);
        page.photos.into_iter().map(|p| p.name).collect()
    }

    fn variada() -> (tempfile::TempDir, Library) {
        biblioteca(&[
            Foto {
                rel: "a/playa.jpg",
                camera: Some("Canon EOS 80D"),
                lens: Some("EF-S 18-55mm"),
                rating: 5,
                favorite: true,
                ..BASE
            },
            Foto {
                rel: "a/playa.cr2",
                local: JULIO_14 - DIA,
                camera: Some("Canon EOS 80D"),
                lens: Some("EF 50mm"),
                rating: 3,
                ..BASE
            },
            Foto {
                rel: "a/monte.mp4",
                kind: "video",
                local: JULIO_14 - 2 * DIA,
                camera: Some("Pixel 7"),
                ..BASE
            },
            Foto {
                rel: "a/monte.jpg",
                local: JULIO_14 - 40 * DIA,
                camera: Some("Pixel 7"),
                lens: Some("Pixel 7 back camera"),
                rating: 1,
                ..BASE
            },
            Foto {
                rel: "a/borrada.jpg",
                camera: Some("Canon EOS 80D"),
                rating: 5,
                favorite: true,
                trashed: true,
                ..BASE
            },
        ])
    }

    #[test]
    fn sin_filtros_ni_texto_salen_todas_menos_la_papelera() {
        let (_d, lib) = variada();
        assert_eq!(buscar(&lib, "", SearchFilters::default()).len(), 4);
        assert!(SearchQuery::parse("").is_empty());
        let con_filtro = SearchQuery {
            filters: SearchFilters {
                favorites: true,
                ..SearchFilters::default()
            },
            ..SearchQuery::default()
        };
        assert!(!con_filtro.is_empty());
    }

    #[test]
    fn tipo() {
        let (_d, lib) = variada();
        let tipo = |kind| {
            buscar(
                &lib,
                "",
                SearchFilters {
                    kind: Some(kind),
                    ..SearchFilters::default()
                },
            )
        };
        assert_eq!(
            tipo(KindFilter::Photos),
            ["playa.jpg", "playa.cr2", "monte.jpg"]
        );
        assert_eq!(tipo(KindFilter::Videos), ["monte.mp4"]);
        assert_eq!(tipo(KindFilter::Raw), ["playa.cr2"]);
    }

    #[test]
    fn camara_y_objetivo() {
        let (_d, lib) = variada();
        let canon = SearchFilters {
            camera: Some("Canon EOS 80D".into()),
            ..SearchFilters::default()
        };
        assert_eq!(buscar(&lib, "", canon.clone()), ["playa.jpg", "playa.cr2"]);
        let cincuenta = SearchFilters {
            lens: Some("EF 50mm".into()),
            ..canon
        };
        assert_eq!(buscar(&lib, "", cincuenta), ["playa.cr2"]);
        // Una comilla o un comodín no son SQL: no encuentran nada.
        for raro in ["' OR 1=1 --", "%", "Canon%"] {
            let f = SearchFilters {
                camera: Some(raro.into()),
                ..SearchFilters::default()
            };
            assert!(buscar(&lib, "", f).is_empty(), "{raro}");
        }
    }

    #[test]
    fn fechas_por_dia_local_con_los_extremos_incluidos() {
        let (_d, lib) = variada();
        let rango = |from: Option<&str>, to: Option<&str>| {
            buscar(
                &lib,
                "",
                SearchFilters {
                    from: from.map(Into::into),
                    to: to.map(Into::into),
                    ..SearchFilters::default()
                },
            )
        };
        assert_eq!(
            rango(Some("2024-07-12"), Some("2024-07-13")),
            ["playa.cr2", "monte.mp4"]
        );
        assert_eq!(rango(Some("2024-07-14"), None), ["playa.jpg"]);
        assert_eq!(rango(None, Some("2024-06-30")), ["monte.jpg"]);
        assert!(rango(Some("2024-07-15"), Some("2024-07-13")).is_empty());
        for malo in ["2024-02-30", "14/07/2024", "2024-7-1", "", "x"] {
            let f = SearchFilters {
                from: Some(malo.into()),
                ..SearchFilters::default()
            };
            let query = SearchQuery {
                filters: f,
                ..SearchQuery::default()
            };
            assert!(
                matches!(lib.search_months(&query), Err(Error::InvalidDay(_))),
                "{malo}"
            );
        }
    }

    #[test]
    fn la_hora_de_pared_decide_el_dia() {
        // 23:30 del 14 de julio en hora de la cámara: es del 14, en cualquier
        // zona del equipo.
        let noche = JULIO_14 + 11 * 3600 + 1800;
        let (_d, lib) = biblioteca(&[Foto {
            rel: "noche.jpg",
            local: noche,
            ..BASE
        }]);
        let dia = |d: &str| SearchFilters {
            from: Some(d.into()),
            to: Some(d.into()),
            ..SearchFilters::default()
        };
        assert_eq!(buscar(&lib, "", dia("2024-07-14")), ["noche.jpg"]);
        assert!(buscar(&lib, "", dia("2024-07-15")).is_empty());
    }

    #[test]
    fn valoracion_minima_y_favoritas() {
        let (_d, lib) = variada();
        let minimo = |n| {
            buscar(
                &lib,
                "",
                SearchFilters {
                    min_rating: Some(n),
                    ..SearchFilters::default()
                },
            )
        };
        assert_eq!(minimo(0).len(), 4);
        assert_eq!(minimo(3), ["playa.jpg", "playa.cr2"]);
        assert_eq!(minimo(5), ["playa.jpg"]);
        let query = SearchQuery {
            filters: SearchFilters {
                min_rating: Some(6),
                ..SearchFilters::default()
            },
            ..SearchQuery::default()
        };
        assert!(matches!(
            lib.search_months(&query),
            Err(Error::InvalidRating(6))
        ));
        let favoritas = SearchFilters {
            favorites: true,
            ..SearchFilters::default()
        };
        assert_eq!(buscar(&lib, "", favoritas), ["playa.jpg"]);
    }

    #[test]
    fn album_y_etiqueta() {
        let (_d, mut lib) = variada();
        let album = lib.create_album("Verano").unwrap();
        lib.add_to_album(album, &[1, 3]).unwrap();
        let tag = lib.create_tag("Familia").unwrap();
        lib.add_tag(&[3, 4], tag.id).unwrap();
        let en_album = SearchFilters {
            album: Some(album),
            ..SearchFilters::default()
        };
        assert_eq!(
            buscar(&lib, "", en_album.clone()),
            ["playa.jpg", "monte.mp4"]
        );
        let con_etiqueta = SearchFilters {
            tag: Some(tag.id),
            ..SearchFilters::default()
        };
        assert_eq!(buscar(&lib, "", con_etiqueta), ["monte.mp4", "monte.jpg"]);
        let ambos = SearchFilters {
            tag: Some(tag.id),
            ..en_album
        };
        assert_eq!(buscar(&lib, "", ambos), ["monte.mp4"]);
    }

    #[test]
    fn se_combinan_con_el_texto_y_con_su_fecha() {
        let (_d, lib) = variada();
        let canon = SearchFilters {
            camera: Some("Canon EOS 80D".into()),
            ..SearchFilters::default()
        };
        assert_eq!(
            buscar(&lib, "playa", canon.clone()),
            ["playa.jpg", "playa.cr2"]
        );
        assert!(buscar(&lib, "monte", canon.clone()).is_empty());
        // Fecha en el texto y rango de fechas: deben cumplirse las dos.
        let julio = SearchFilters {
            from: Some("2024-07-01".into()),
            to: Some("2024-07-13".into()),
            ..canon
        };
        assert_eq!(buscar(&lib, "julio 2024", julio.clone()), ["playa.cr2"]);
        assert!(buscar(&lib, "junio 2024", julio).is_empty());
        // Todo a la vez.
        let todo = SearchFilters {
            kind: Some(KindFilter::Photos),
            camera: Some("Canon EOS 80D".into()),
            lens: Some("EF-S 18-55mm".into()),
            from: Some("2024-07-14".into()),
            to: Some("2024-07-14".into()),
            min_rating: Some(4),
            favorites: true,
            album: None,
            tag: None,
        };
        assert_eq!(buscar(&lib, "playa", todo), ["playa.jpg"]);
    }

    #[test]
    fn seleccionar_todo_respeta_los_filtros() {
        let (_d, lib) = variada();
        let query = SearchQuery {
            filters: SearchFilters {
                kind: Some(KindFilter::Videos),
                ..SearchFilters::default()
            },
            ..SearchQuery::default()
        };
        assert_eq!(lib.select_all(crate::Scope::Search(&query)).unwrap(), [3]);
    }

    #[test]
    fn opciones_del_panel_sin_papelera() {
        let (_d, mut lib) = variada();
        let album = lib.create_album("Verano").unwrap();
        lib.add_to_album(album, &[1, 5]).unwrap();
        lib.create_album("animales").unwrap();
        let tag = lib.create_tag("Familia").unwrap();
        lib.add_tag(&[3, 4, 5], tag.id).unwrap();
        let o = lib.filter_options().unwrap();
        assert_eq!(
            o.cameras,
            [
                CameraOption {
                    name: "Canon EOS 80D".into(),
                    photos: 2
                },
                CameraOption {
                    name: "Pixel 7".into(),
                    photos: 2
                },
            ]
        );
        let objetivos: Vec<(Option<&str>, &str, u64)> = o
            .lenses
            .iter()
            .map(|l| (l.camera.as_deref(), l.name.as_str(), l.photos))
            .collect();
        assert_eq!(
            objetivos,
            [
                (Some("Canon EOS 80D"), "EF 50mm", 1),
                (Some("Canon EOS 80D"), "EF-S 18-55mm", 1),
                (Some("Pixel 7"), "Pixel 7 back camera", 1),
            ]
        );
        let albumes: Vec<(&str, u64)> = o
            .albums
            .iter()
            .map(|a| (a.name.as_str(), a.photos))
            .collect();
        assert_eq!(albumes, [("animales", 0), ("Verano", 1)]);
        assert_eq!(o.tags.len(), 1);
        assert_eq!((o.tags[0].id, o.tags[0].photos), (tag.id, 2));
    }

    #[test]
    fn se_serializa_en_espanol() {
        let f: SearchFilters = serde_json::from_str(
            r#"{"tipo":"raw","camara":"X","desde":"2024-01-01","valoracion_minima":3,"favoritas":true,"etiqueta":2}"#,
        )
        .unwrap();
        assert_eq!(f.kind, Some(KindFilter::Raw));
        assert_eq!(f.min_rating, Some(3));
        assert_eq!(f.tag, Some(2));
        assert!(f.lens.is_none() && f.album.is_none());
        let vacio: SearchFilters = serde_json::from_str("{}").unwrap();
        assert!(vacio.is_empty());
    }
}
