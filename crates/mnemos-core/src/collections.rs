//! Colecciones automáticas de la pantalla Explorar (spec 021): vídeos,
//! capturas de pantalla, panorámicas, editadas, RAW y añadidas recientemente.
//!
//! Son vistas calculadas con columnas que ya existen: no guardan nada ni
//! tocan archivos. Igual que Fotos, dejan fuera las archivadas y la Papelera.

use serde::{Deserialize, Serialize};

use crate::timeline::{DATE, Filter};

/// Una colección automática de Explorar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Collection {
    /// `media_kind = 'video'` (spec 019).
    #[serde(rename = "videos")]
    Videos,
    /// Por el nombre del archivo: ver [`SCREENSHOT_PATTERNS`].
    #[serde(rename = "capturas")]
    Screenshots,
    /// Fotos con el lado largo al menos el doble que el corto.
    #[serde(rename = "panoramicas")]
    Panoramas,
    /// Fotos con una edición a la vista (spec 009).
    #[serde(rename = "editadas")]
    Edited,
    /// Fotos RAW o DNG, por la extensión.
    #[serde(rename = "raw")]
    Raw,
    /// Todas, por la fecha de importación: en esta colección,
    /// `GridPhoto::date` es esa fecha, como en la Papelera.
    #[serde(rename = "recientes")]
    Recent,
}

impl Collection {
    /// Todas, en el orden de la fila «Colecciones».
    pub const ALL: [Collection; 6] = [
        Collection::Videos,
        Collection::Screenshots,
        Collection::Panoramas,
        Collection::Edited,
        Collection::Raw,
        Collection::Recent,
    ];
}

/// Patrones `LIKE` (con `\` de escape) sobre el nombre del archivo en
/// minúsculas que marcan una captura o grabación de pantalla. Es la única
/// lista: para reconocer otro nombre, se añade aquí.
///
/// `lower()` de SQLite solo cambia letras ASCII, por eso «grabación» va con
/// y sin tilde.
pub const SCREENSHOT_PATTERNS: &[&str] = &[
    "screenshot%",
    "screen shot%",
    "screen\\_recording%",
    "screenrecorder%",
    "captura%",
    "grabación de pantalla%",
    "grabacion de pantalla%",
    "scr\\_%",
    "%\\_screenshot%",
];

/// Nombre del archivo (lo que hay tras la última `/`) de `rel_path`, en SQL.
const FILE_NAME: &str = "replace(rel_path, rtrim(rel_path, replace(rel_path, '/', '')), '')";

/// Lo que comparten todas las colecciones: sin archivadas ni Papelera.
const BASE: &str = " AND archived_at IS NULL AND trashed_at IS NULL";

impl Collection {
    pub(crate) fn filter(self) -> Filter {
        let condition = match self {
            Collection::Recent => {
                return Filter {
                    sql: BASE.to_owned(),
                    params: Vec::new(),
                    order: "imported_at",
                };
            }
            Collection::Videos => "media_kind = 'video'".to_owned(),
            Collection::Screenshots => {
                // Sacar el nombre del archivo en SQL es caro: primero se
                // descartan las rutas que no contienen ningún patrón (un
                // `LIKE` simple) y solo a las que quedan se les mira el nombre.
                let somewhere = SCREENSHOT_PATTERNS
                    .iter()
                    .map(|p| {
                        let p = p.trim_start_matches('%');
                        format!("lower(rel_path) LIKE '%{p}' ESCAPE '\\'")
                    })
                    .collect::<Vec<_>>()
                    .join(" OR ");
                let name = SCREENSHOT_PATTERNS
                    .iter()
                    .map(|p| format!("lower({FILE_NAME}) LIKE '{p}' ESCAPE '\\'"))
                    .collect::<Vec<_>>()
                    .join(" OR ");
                format!("({somewhere}) AND ({name})")
            }
            Collection::Panoramas => "media_kind = 'foto' AND width > 0 AND height > 0 \
                 AND max(width, height) >= 2 * min(width, height)"
                .to_owned(),
            Collection::Edited => "media_kind = 'foto' AND edit_version IS NOT NULL".to_owned(),
            Collection::Raw => {
                let any = mnemos_imaging::RAW_EXTENSIONS
                    .iter()
                    .map(|ext| format!("lower(rel_path) LIKE '%.{ext}'"))
                    .collect::<Vec<_>>()
                    .join(" OR ");
                format!("({any})")
            }
        };
        Filter {
            sql: format!("{BASE} AND {condition}"),
            params: Vec::new(),
            order: DATE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;
    use crate::timeline::View;
    use rusqlite::params;

    const OCT_2026: i64 = 1_791_892_800;

    struct Fila<'a> {
        rel: &'a str,
        kind: &'a str,
        w: u32,
        h: u32,
        edit: Option<u32>,
        archived: bool,
        trashed: bool,
    }

    fn fila(rel: &str) -> Fila<'_> {
        Fila {
            rel,
            kind: "foto",
            w: 4000,
            h: 3000,
            edit: None,
            archived: false,
            trashed: false,
        }
    }

    fn library(rows: &[Fila]) -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        for (i, f) in rows.iter().enumerate() {
            lib.conn
                .execute(
                    "INSERT INTO photos (rel_path, size_bytes, modified_at, width, height,
                                         imported_at, media_kind, edit_version, archived_at,
                                         trashed_at)
                     VALUES (?1, 1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8)",
                    params![
                        f.rel,
                        OCT_2026 - i as i64,
                        f.w,
                        f.h,
                        f.kind,
                        f.edit,
                        f.archived.then_some(OCT_2026),
                        f.trashed.then_some(OCT_2026),
                    ],
                )
                .unwrap();
        }
        (dir, lib)
    }

    fn nombres(lib: &Library, c: Collection) -> Vec<String> {
        lib.view_page(View::Collection(c), None, 100)
            .unwrap()
            .photos
            .into_iter()
            .map(|p| p.name)
            .collect()
    }

    #[test]
    fn videos_solo_los_videos() {
        let (_d, lib) = library(&[
            Fila {
                kind: "video",
                ..fila("Originales/2026/10/paseo.mp4")
            },
            fila("Originales/2026/10/foto.jpg"),
        ]);
        assert_eq!(nombres(&lib, Collection::Videos), ["paseo.mp4"]);
    }

    #[test]
    fn capturas_por_el_nombre_del_archivo() {
        let (_d, lib) = library(&[
            fila("Originales/2026/10/Screenshot_20261012-101010.png"),
            fila("Originales/2026/10/Screen Shot 2026-10-12 at 10.10.10.png"),
            fila("Originales/2026/10/Captura de pantalla 2026-10-12.png"),
            fila("Originales/2026/10/Grabación de pantalla 2026-10-12.mov"),
            fila("Originales/2026/10/SCREENSHOT.PNG"),
            fila("Originales/2026/10/IMG_1234_screenshot.jpg"),
            fila("Originales/2026/10/Scr_2026.png"),
            // No son capturas: el patrón mira solo el nombre, no la carpeta,
            // y `_` no es un comodín.
            fila("Originales/Screenshots/playa.jpg"),
            fila("Originales/2026/10/scrabble.jpg"),
            fila("Originales/2026/10/IMG_1235.jpg"),
        ]);
        assert_eq!(
            nombres(&lib, Collection::Screenshots),
            [
                "Screenshot_20261012-101010.png",
                "Screen Shot 2026-10-12 at 10.10.10.png",
                "Captura de pantalla 2026-10-12.png",
                "Grabación de pantalla 2026-10-12.mov",
                "SCREENSHOT.PNG",
                "IMG_1234_screenshot.jpg",
                "Scr_2026.png",
            ]
        );
    }

    #[test]
    fn panoramicas_horizontales_y_verticales_sin_videos() {
        let (_d, lib) = library(&[
            Fila {
                w: 8000,
                h: 4000,
                ..fila("Originales/ancha.jpg")
            },
            Fila {
                w: 1000,
                h: 2500,
                ..fila("Originales/alta.jpg")
            },
            Fila {
                w: 7999,
                h: 4000,
                ..fila("Originales/casi.jpg")
            },
            Fila {
                w: 0,
                h: 0,
                ..fila("Originales/sin-medidas.jpg")
            },
            Fila {
                kind: "video",
                w: 3840,
                h: 1600,
                ..fila("Originales/cine.mp4")
            },
        ]);
        assert_eq!(
            nombres(&lib, Collection::Panoramas),
            ["ancha.jpg", "alta.jpg"]
        );
    }

    #[test]
    fn editadas_y_raw() {
        let (_d, lib) = library(&[
            Fila {
                edit: Some(2),
                ..fila("Originales/editada.jpg")
            },
            fila("Originales/sin-editar.jpg"),
            fila("Originales/DSC0001.ARW"),
            fila("Originales/IMG_0001.dng"),
            fila("Originales/arw.jpg"),
        ]);
        assert_eq!(nombres(&lib, Collection::Edited), ["editada.jpg"]);
        assert_eq!(
            nombres(&lib, Collection::Raw),
            ["DSC0001.ARW", "IMG_0001.dng"]
        );
    }

    #[test]
    fn sin_archivadas_ni_papelera() {
        let (_d, lib) = library(&[
            Fila {
                kind: "video",
                ..fila("Originales/a.mp4")
            },
            Fila {
                kind: "video",
                archived: true,
                ..fila("Originales/archivado.mp4")
            },
            Fila {
                kind: "video",
                trashed: true,
                ..fila("Originales/papelera.mp4")
            },
        ]);
        assert_eq!(nombres(&lib, Collection::Videos), ["a.mp4"]);
        let meses = lib
            .view_months(View::Collection(Collection::Videos))
            .unwrap();
        assert_eq!(meses.iter().map(|m| m.photos).sum::<u64>(), 1);
    }

    #[test]
    fn seleccionar_todo_en_una_coleccion() {
        let (_d, lib) = library(&[
            Fila {
                kind: "video",
                ..fila("Originales/a.mp4")
            },
            fila("Originales/b.jpg"),
            Fila {
                kind: "video",
                ..fila("Originales/c.mp4")
            },
        ]);
        let scope = crate::selection::Scope::View(View::Collection(Collection::Videos));
        assert_eq!(lib.select_all(scope).unwrap().len(), 2);
    }

    #[test]
    fn se_serializa_como_coleccion() {
        let v: View = serde_json::from_str(r#"{"coleccion":"capturas"}"#).unwrap();
        assert_eq!(v, View::Collection(Collection::Screenshots));
        assert_eq!(
            serde_json::to_string(&View::Collection(Collection::Panoramas)).unwrap(),
            r#"{"coleccion":"panoramicas"}"#
        );
    }

    /// Fila con fecha de captura e importación propias.
    fn insertar(lib: &Library, rel: &str, taken: Option<i64>, imported: i64, archived: bool) {
        lib.conn
            .execute(
                "INSERT INTO photos (rel_path, size_bytes, modified_at, taken_at, imported_at,
                                     archived_at)
                 VALUES (?1, 1, ?2, ?3, ?4, ?5)",
                params![rel, OCT_2026, taken, imported, archived.then_some(OCT_2026)],
            )
            .unwrap();
    }

    #[test]
    fn recientes_por_fecha_de_importacion() {
        let (_d, lib) = library(&[]);
        const DIA: i64 = 86_400;
        // Foto antigua importada ayer y foto nueva importada hace un mes.
        insertar(
            &lib,
            "Originales/antigua.jpg",
            Some(OCT_2026 - 3000 * DIA),
            OCT_2026 - DIA,
            false,
        );
        insertar(
            &lib,
            "Originales/nueva.jpg",
            Some(OCT_2026),
            OCT_2026 - 30 * DIA,
            false,
        );
        insertar(&lib, "Originales/hoy.jpg", None, OCT_2026, false);
        insertar(&lib, "Originales/archivada.jpg", None, OCT_2026, true);

        let vista = View::Collection(Collection::Recent);
        let pagina = lib.view_page(vista, None, 100).unwrap();
        let nombres: Vec<_> = pagina.photos.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(nombres, ["hoy.jpg", "antigua.jpg", "nueva.jpg"]);
        // La fecha de la cuadrícula es la de importación, para agrupar por día.
        assert_eq!(pagina.photos[1].date, OCT_2026 - DIA);

        let meses = lib.view_months(vista).unwrap();
        assert_eq!(meses.iter().map(|m| m.photos).sum::<u64>(), 3);
        let scope = crate::selection::Scope::View(vista);
        assert_eq!(lib.select_all(scope).unwrap().len(), 3);
    }

    #[test]
    fn rango_de_un_recuerdo() {
        let (_d, lib) = library(&[]);
        let desde = OCT_2026;
        let hasta = OCT_2026 + 7 * 86_400;
        insertar(&lib, "Originales/primera.jpg", Some(desde), 0, false);
        insertar(
            &lib,
            "Originales/dentro.jpg",
            Some(desde + 3 * 86_400),
            0,
            false,
        );
        insertar(&lib, "Originales/justo-fuera.jpg", Some(hasta), 0, false);
        insertar(&lib, "Originales/antes.jpg", Some(desde - 1), 0, false);
        insertar(&lib, "Originales/archivada.jpg", Some(desde + 60), 0, true);
        // Sin fecha de captura: su fecha de modificación cae dentro, pero no
        // cuenta.
        insertar(&lib, "Originales/sin-fecha.jpg", None, 0, false);

        let vista = View::Range { desde, hasta };
        let nombres: Vec<_> = lib
            .view_page(vista, None, 100)
            .unwrap()
            .photos
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(nombres, ["dentro.jpg", "primera.jpg"]);
    }

    #[test]
    fn rango_y_recientes_se_serializan() {
        let v: View = serde_json::from_str(r#"{"rango":{"desde":10,"hasta":20}}"#).unwrap();
        assert_eq!(
            v,
            View::Range {
                desde: 10,
                hasta: 20
            }
        );
        assert_eq!(
            serde_json::to_string(&View::Collection(Collection::Recent)).unwrap(),
            r#"{"coleccion":"recientes"}"#
        );
    }
}
