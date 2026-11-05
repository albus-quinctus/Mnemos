//! Metadatos de Google Takeout (specs 004 y 007): el `.json` que acompaña a
//! cada foto y el que describe la carpeta de un álbum.
//!
//! Solo se leen archivos del origen de la importación y solo se escribe en la
//! base de datos.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::error::Result;

/// Lo que se lee del `.json` de una foto.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PhotoMeta {
    /// `photoTakenTime`, en segundos Unix.
    pub taken_at: Option<i64>,
    pub description: Option<String>,
    pub geo: Option<Geo>,
    pub favorited: bool,
    pub archived: bool,
}

/// Ubicación en grados y altitud en metros.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Geo {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

/// Una descripción más larga se corta.
const MAX_DESCRIPTION: usize = 10_000;

/// Los `.json` de Takeout ocupan unos cientos de bytes; uno enorme no es suyo.
const MAX_JSON: u64 = 1 << 20;

/// Los `.json` de cada carpeta del origen, leídos una sola vez por carpeta.
#[derive(Debug, Default)]
pub(crate) struct Sidecars {
    dirs: HashMap<PathBuf, Dir>,
}

#[derive(Debug)]
struct Dir {
    jsons: Vec<String>,
    /// Título del álbum, si la carpeta es un álbum.
    album: Option<String>,
}

/// Sufijos que Google Fotos añade a las copias editadas, que no tienen `.json`
/// propio: se usa el del original.
const EDITED_SUFFIXES: &[&str] = &["-editada", "-editado", "-edited"];

const SUPPLEMENTAL: &str = ".supplemental-metadata";

impl Sidecars {
    /// Metadatos del `.json` de la foto, si lo tiene y se puede leer.
    pub(crate) fn photo(&mut self, photo: &Path) -> Option<PhotoMeta> {
        let dir = photo.parent()?;
        let name = photo.file_name()?.to_str()?;
        let json = sidecar_for(name, &self.dir(dir).jsons)?;
        read_photo_meta(&dir.join(json))
    }

    /// Título del álbum de la carpeta en la que está la foto.
    pub(crate) fn album(&mut self, photo: &Path) -> Option<String> {
        self.dir(photo.parent()?).album.clone()
    }

    fn dir(&mut self, dir: &Path) -> &Dir {
        self.dirs.entry(dir.to_owned()).or_insert_with(|| {
            let jsons: Vec<String> = fs::read_dir(dir)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter(|n| n.to_ascii_lowercase().ends_with(".json"))
                .collect();
            let album = album_title(dir, &jsons);
            Dir { jsons, album }
        })
    }
}

/// Nombre del `.json` de `photo` entre los de su carpeta.
///
/// Takeout lo llama `foto.jpg.supplemental-metadata.json` (o `foto.jpg.json`
/// en exportaciones antiguas), lo trunca si es muy largo y, en las copias
/// `foto(1).jpg`, mueve el número al final: `foto.jpg.supplemental-metadata(1).json`.
fn sidecar_for<'a>(photo: &str, jsons: &'a [String]) -> Option<&'a str> {
    let (stem, ext) = match photo.rfind('.') {
        Some(dot) => (&photo[..dot], &photo[dot..]),
        None => (photo, ""),
    };
    let mut stems = vec![stem];
    stems.extend(EDITED_SUFFIXES.iter().filter_map(|s| stem.strip_suffix(s)));
    // Primero el nombre tal cual (`foto (1).jpg.supplemental-metadata.json`),
    // después sin el número de copia (`foto.jpg.supplemental-metadata(1).json`).
    let candidates = stems.into_iter().flat_map(|stem| {
        let (core, copy) = split_copy(stem);
        [(stem, None), (core.trim_end(), copy)]
    });
    for (core, copy) in candidates {
        let full = format!("{core}{ext}");
        let long = format!("{full}{SUPPLEMENTAL}");
        let found = jsons.iter().find(|json| {
            let base = &json[..json.len() - ".json".len()];
            let (base, json_copy) = split_copy(base);
            json_copy == copy
                && (base == full
                    || (base.len() > full.len() && long.starts_with(base))
                    // Nombre tan largo que Takeout lo cortó antes de la extensión.
                    || (base.len() >= 40 && full.starts_with(base)))
        });
        if let Some(json) = found {
            return Some(json);
        }
    }
    None
}

/// Separa el número de copia final: `foto(1)` → (`foto`, `Some("1")`).
fn split_copy(stem: &str) -> (&str, Option<&str>) {
    if let Some(open) = stem.strip_suffix(')').and_then(|s| s.rfind('(')) {
        let n = &stem[open + 1..stem.len() - 1];
        if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) {
            return (&stem[..open], Some(n));
        }
    }
    (stem, None)
}

fn read_json(path: &Path) -> Option<Value> {
    if fs::metadata(path).ok()?.len() > MAX_JSON {
        return None;
    }
    let value: Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    value.is_object().then_some(value)
}

/// Lee el `.json` de una foto. Cada campo se lee por separado: uno con un tipo
/// inesperado no impide leer los demás.
fn read_photo_meta(path: &Path) -> Option<PhotoMeta> {
    let json = read_json(path)?;
    let taken_at = json["photoTakenTime"]["timestamp"]
        .as_str()
        .and_then(|t| t.parse::<i64>().ok())
        .filter(|&secs| secs > 0);
    Some(PhotoMeta {
        taken_at,
        description: json["description"].as_str().and_then(clean_description),
        geo: geo(&json["geoData"]).or_else(|| geo(&json["geoDataExif"])),
        favorited: json["favorited"].as_bool() == Some(true),
        archived: json["archived"].as_bool() == Some(true),
    })
}

fn clean_description(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    Some(match text.char_indices().nth(MAX_DESCRIPTION) {
        Some((end, _)) => text[..end].trim_end().to_owned(),
        None => text.to_owned(),
    })
}

/// Ubicación válida de un objeto `geoData`. Google escribe `0.0` cuando no hay.
fn geo(value: &Value) -> Option<Geo> {
    let latitude = value["latitude"].as_f64()?;
    let longitude = value["longitude"].as_f64()?;
    let valid = (-90.0..=90.0).contains(&latitude)
        && (-180.0..=180.0).contains(&longitude)
        && (latitude, longitude) != (0.0, 0.0);
    valid.then(|| Geo {
        latitude,
        longitude,
        // Google escribe 0.0 cuando no conoce la altitud.
        altitude: value["altitude"]
            .as_f64()
            .filter(|a| a.is_finite() && *a != 0.0),
    })
}

/// Título del álbum de una carpeta de Takeout.
///
/// El `.json` del álbum tiene un nombre que depende del idioma
/// (`metadata.json`, `metadatos.json`…) y, a diferencia de los de las fotos,
/// no lleva la extensión de una imagen: no tiene ningún punto antes de `.json`.
/// Se reconoce por tener `title` y no tener `photoTakenTime`.
fn album_title(dir: &Path, jsons: &[String]) -> Option<String> {
    if is_year_folder(&dir.file_name()?.to_string_lossy()) {
        return None;
    }
    jsons
        .iter()
        .filter(|name| !name[..name.len() - ".json".len()].contains('.'))
        .filter_map(|name| read_json(&dir.join(name)))
        .filter(|json| json.get("photoTakenTime").is_none())
        .filter_map(|json| Some(json["title"].as_str()?.trim().to_owned()))
        .find(|title| !title.is_empty() && !is_year_folder(title))
}

/// Carpetas por año de Takeout: `Photos from 2019`, `Fotos de 2019`.
fn is_year_folder(name: &str) -> bool {
    let Some((prefix, year)) = name.trim().rsplit_once(' ') else {
        return false;
    };
    let year_ok = year.len() == 4 && year.bytes().all(|b| b.is_ascii_digit());
    let prefix = prefix.trim().to_lowercase();
    year_ok && ["photos from", "fotos de", "fotos del"].contains(&prefix.as_str())
}

/// Aplica a las fotos con esta huella los metadatos de su `.json` y su álbum.
///
/// La descripción, la ubicación, la favorita y el archivo se aplican una sola
/// vez por foto (`takeout_at`), para no deshacer lo que el usuario cambie
/// después. El álbum se añade siempre, sin repetir entradas.
pub(crate) fn apply(
    conn: &Connection,
    hash: &str,
    meta: Option<&PhotoMeta>,
    album: Option<&str>,
) -> Result<()> {
    if let Some(meta) = meta {
        let geo = meta.geo;
        conn.execute(
            "UPDATE photos SET
                 description = coalesce(description, ?2),
                 latitude    = CASE WHEN latitude IS NULL THEN ?3 ELSE latitude END,
                 longitude   = CASE WHEN latitude IS NULL THEN ?4 ELSE longitude END,
                 altitude    = CASE WHEN latitude IS NULL THEN ?5 ELSE altitude END,
                 favorite    = max(favorite, ?6),
                 archived_at = CASE WHEN ?7 THEN coalesce(archived_at, unixepoch())
                                    ELSE archived_at END,
                 takeout_at  = unixepoch()
             WHERE content_hash = ?1 AND takeout_at IS NULL",
            params![
                hash,
                meta.description,
                geo.map(|g| g.latitude),
                geo.map(|g| g.longitude),
                geo.and_then(|g| g.altitude),
                meta.favorited,
                meta.archived,
            ],
        )?;
    }
    if let Some(name) = album {
        let album_id = album_id(conn, name)?;
        conn.execute(
            "INSERT OR IGNORE INTO album_photos (album_id, photo_id, position)
             SELECT ?1, id, (SELECT coalesce(max(position) + 1, 0)
                             FROM album_photos WHERE album_id = ?1)
             FROM photos WHERE content_hash = ?2",
            params![album_id, hash],
        )?;
    }
    Ok(())
}

/// Id del álbum con este nombre; si no existe, lo crea.
fn album_id(conn: &Connection, name: &str) -> Result<i64> {
    let found = conn
        .query_row(
            "SELECT id FROM albums WHERE name = ?1 ORDER BY id LIMIT 1",
            [name],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = found {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO albums (name, created_at) VALUES (?1, unixepoch())",
        [name],
    )?;
    Ok(conn.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encuentra_el_json_de_takeout() {
        let jsons: Vec<String> = [
            "IMG-20181215-WA0003.jpg.supplemental-metadata.json",
            "IMG20220110212038.jpg.supplemental-metadata(1).json",
            "IMG20220110212038.jpg.supplemental-metadata.json",
            "antigua.jpg.json",
            "nombre muy largo que google takeout corta antes de.json",
            "foto.png.supplemental-me.json",
            "metadatos.json",
            "Untitled (1).png.supplemental-metadata.json",
        ]
        .map(String::from)
        .into();
        let casos = [
            ("IMG-20181215-WA0003.jpg", Some(&jsons[0])),
            ("IMG-20181215-WA0003-editada.jpg", Some(&jsons[0])),
            ("IMG20220110212038(1).jpg", Some(&jsons[1])),
            ("IMG20220110212038.jpg", Some(&jsons[2])),
            ("antigua.jpg", Some(&jsons[3])),
            (
                "nombre muy largo que google takeout corta antes de la extension.jpg",
                Some(&jsons[4]),
            ),
            ("foto.png", Some(&jsons[5])),
            ("Untitled (1).png", Some(&jsons[7])),
            ("foto.jpg", None),
            ("otra.jpg", None),
        ];
        for (foto, json) in casos {
            assert_eq!(
                sidecar_for(foto, &jsons),
                json.map(String::as_str),
                "{foto}"
            );
        }
    }

    fn meta_of(json: &str) -> Option<PhotoMeta> {
        let dir = tempfile::tempdir().unwrap();
        let foto = dir.path().join("IMG_1.jpg");
        fs::write(
            dir.path().join("IMG_1.jpg.supplemental-metadata.json"),
            json,
        )
        .unwrap();
        Sidecars::default().photo(&foto)
    }

    #[test]
    fn lee_photo_taken_time_e_ignora_json_raros() {
        let meta = meta_of(
            r#"{"title":"IMG_1.jpg","creationTime":{"timestamp":"1715871078"},
               "photoTakenTime":{"timestamp":"1679579100","formatted":"23 mar 2023"}}"#,
        )
        .unwrap();
        assert_eq!(meta.taken_at, Some(1_679_579_100));

        for raro in ["no es json", "[]"] {
            assert_eq!(meta_of(raro), None, "{raro}");
        }
        for sin_fecha in ["{}", r#"{"photoTakenTime":{"timestamp":"0"}}"#] {
            assert_eq!(meta_of(sin_fecha).unwrap().taken_at, None, "{sin_fecha}");
        }
    }

    #[test]
    fn lee_descripcion_ubicacion_favorita_y_archivo() {
        let meta = meta_of(
            r#"{"title":"IMG_1.jpg","description":"  Subida al Naranjo \n",
               "photoTakenTime":{"timestamp":"1557664200"},
               "geoData":{"latitude":43.2,"longitude":-4.8,"altitude":2519.0,
                          "latitudeSpan":0.0,"longitudeSpan":0.0},
               "geoDataExif":{"latitude":1.0,"longitude":1.0,"altitude":0.0},
               "favorited":true,"archived":true}"#,
        )
        .unwrap();
        assert_eq!(
            meta,
            PhotoMeta {
                taken_at: Some(1_557_664_200),
                description: Some("Subida al Naranjo".into()),
                geo: Some(Geo {
                    latitude: 43.2,
                    longitude: -4.8,
                    altitude: Some(2519.0)
                }),
                favorited: true,
                archived: true,
            }
        );
    }

    #[test]
    fn sin_geo_data_usa_el_del_exif_y_a_cero_no_hay_ubicacion() {
        let cero = r#"{"latitude":0.0,"longitude":0.0,"altitude":0.0}"#;
        let exif = r#"{"latitude":40.4,"longitude":-3.7,"altitude":650.0}"#;
        let meta = meta_of(&format!(r#"{{"geoData":{cero},"geoDataExif":{exif}}}"#)).unwrap();
        assert_eq!(meta.geo.map(|g| g.latitude), Some(40.4));

        let meta = meta_of(&format!(r#"{{"geoData":{cero},"geoDataExif":{cero}}}"#)).unwrap();
        assert_eq!(meta.geo, None);

        let sin_altitud = r#"{"geoData":{"latitude":40.4,"longitude":-3.7,"altitude":0.0}}"#;
        assert_eq!(meta_of(sin_altitud).unwrap().geo.unwrap().altitude, None);

        let fuera = r#"{"geoData":{"latitude":95.0,"longitude":10.0}}"#;
        assert_eq!(meta_of(fuera).unwrap().geo, None);
        let texto = r#"{"geoData":{"latitude":"40","longitude":"3"}}"#;
        assert_eq!(meta_of(texto).unwrap().geo, None);
    }

    #[test]
    fn un_campo_con_otro_tipo_no_impide_leer_los_demas() {
        let meta = meta_of(
            r#"{"description":5,"favorited":"true","archived":false,
               "photoTakenTime":{"timestamp":"1557664200"}}"#,
        )
        .unwrap();
        assert_eq!(
            meta,
            PhotoMeta {
                taken_at: Some(1_557_664_200),
                ..PhotoMeta::default()
            }
        );
    }

    #[test]
    fn descripciones_vacias_o_largas() {
        assert_eq!(clean_description("   \n"), None);
        let larga = "á".repeat(MAX_DESCRIPTION + 5);
        assert_eq!(
            clean_description(&larga).unwrap().chars().count(),
            MAX_DESCRIPTION
        );
    }

    fn album_of(carpeta: &str, archivos: &[(&str, &str)]) -> Option<String> {
        let dir = tempfile::tempdir().unwrap();
        let carpeta = dir.path().join(carpeta);
        fs::create_dir_all(&carpeta).unwrap();
        for (nombre, contenido) in archivos {
            fs::write(carpeta.join(nombre), contenido).unwrap();
        }
        Sidecars::default().album(&carpeta.join("IMG_1.jpg"))
    }

    #[test]
    fn reconoce_las_carpetas_de_album() {
        let foto = (
            "IMG_1.jpg.supplemental-metadata.json",
            r#"{"title":"IMG_1.jpg","photoTakenTime":{"timestamp":"1"}}"#,
        );
        let album = r#"{"title":" Picos de Europa ","description":"","access":"protected",
                        "date":{"timestamp":"1557664200"}}"#;
        assert_eq!(
            album_of("Picos", &[foto, ("metadatos.json", album)]),
            Some("Picos de Europa".into())
        );
        assert_eq!(
            album_of("Picos", &[("metadata.json", album)]),
            Some("Picos de Europa".into())
        );
        // Sin .json de álbum, con título vacío o en una carpeta por año.
        assert_eq!(album_of("Picos", &[foto]), None);
        assert_eq!(
            album_of("Vacío", &[("metadata.json", r#"{"title":"  "}"#)]),
            None
        );
        assert_eq!(
            album_of("Fotos de 2019", &[foto, ("metadatos.json", album)]),
            None
        );
        assert_eq!(
            album_of(
                "Photos from 2019",
                &[("metadata.json", r#"{"title":"Photos from 2019"}"#)]
            ),
            None
        );
    }

    #[test]
    fn carpetas_por_anio() {
        for si in ["Photos from 2019", "Fotos de 2019", "fotos del 2020"] {
            assert!(is_year_folder(si), "{si}");
        }
        for no in ["Viaje 2019", "2019", "Fotos de Asturias", "Fotos de 19"] {
            assert!(!is_year_folder(no), "{no}");
        }
    }
}
