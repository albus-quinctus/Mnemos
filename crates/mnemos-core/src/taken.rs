//! Fecha de captura de una foto (spec 004): EXIF, nombre del archivo o el
//! `.json` que deja Google Takeout.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use mnemos_imaging::CaptureTime;
use rusqlite::{Connection, params};
use serde::Deserialize;

use crate::error::Result;
use crate::files::{days_from_civil, year_month};

/// De dónde salió la fecha, de la más fiable a la menos.
///
/// Un nombre con fecha y hora va antes que el `.json` de Takeout: lo escribe la
/// cámara al hacer la foto, mientras que en las fotos sin EXIF Google a veces
/// guarda la fecha de subida. Un nombre con solo el día (`IMG-20181215-WA0003`)
/// va después.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Source {
    None,
    NameDay,
    Takeout,
    Name,
    Exif,
}

impl Source {
    /// Valor guardado en `photos.taken_source`.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Source::None => "ninguna",
            Source::NameDay => "nombre_dia",
            Source::Takeout => "takeout",
            Source::Name => "nombre",
            Source::Exif => "exif",
        }
    }

    /// Expresión SQL con el orden de `taken_source`: las fuentes que no
    /// conoce (o `NULL`, sin buscar todavía) valen lo mismo que `ninguna`.
    pub(crate) const RANK_SQL: &str = "CASE taken_source
        WHEN 'exif' THEN 4 WHEN 'nombre' THEN 3 WHEN 'takeout' THEN 2 WHEN 'nombre_dia' THEN 1
        ELSE 0 END";

    pub(crate) fn rank(self) -> i64 {
        self as i64
    }
}

/// Momento de la captura.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum When {
    /// Segundos Unix exactos.
    Unix(i64),
    /// Hora local sin zona horaria: la base de datos la pasa a UTC con la zona
    /// del equipo, para que la cuadrícula muestre la hora que marcaba la cámara.
    Local(CaptureTime),
}

/// Fecha de captura y su fuente. `when` solo falta con [`Source::None`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Taken {
    pub when: Option<When>,
    pub source: Source,
}

/// Expresión SQL del valor de `taken_at` a partir de dos parámetros: segundos
/// exactos y hora local (ver [`Taken::sql_params`]).
pub(crate) fn taken_at_sql(exact: usize, local: usize) -> String {
    format!("coalesce(?{exact}, unixepoch(?{local}, 'utc'))")
}

/// Guarda la fecha de captura de la foto `rel_path`.
pub(crate) fn set_taken(conn: &Connection, rel_path: &str, taken: &Taken) -> Result<()> {
    let (exact, local) = taken.sql_params();
    conn.execute(
        &format!(
            "UPDATE photos SET taken_at = {}, taken_source = ?4 WHERE rel_path = ?1",
            taken_at_sql(2, 3)
        ),
        params![rel_path, exact, local, taken.source.as_str()],
    )?;
    Ok(())
}

/// Mejora la fecha de las fotos con esta huella si `taken` sale de una fuente
/// más fiable que la que tienen. Devuelve cuántas cambió.
pub(crate) fn improve_taken(conn: &Connection, hash: &str, taken: &Taken) -> Result<usize> {
    let (exact, local) = taken.sql_params();
    Ok(conn.execute(
        &format!(
            "UPDATE photos SET taken_at = {}, taken_source = ?4
             WHERE content_hash = ?1 AND {} < ?5",
            taken_at_sql(2, 3),
            Source::RANK_SQL
        ),
        params![
            hash,
            exact,
            local,
            taken.source.as_str(),
            taken.source.rank()
        ],
    )?)
}

impl Taken {
    const NONE: Taken = Taken {
        when: None,
        source: Source::None,
    };

    fn new(when: When, source: Source) -> Self {
        let when = match when {
            When::Local(t) => match t.offset_minutes {
                Some(offset) => When::Unix(local_secs(&t) - i64::from(offset) * 60),
                None => When::Local(t),
            },
            unix => unix,
        };
        Taken {
            when: Some(when),
            source,
        }
    }

    /// Fecha de una foto que ya está en la biblioteca: EXIF o nombre.
    pub(crate) fn of_file(path: &Path) -> Self {
        Self::find(path, |_| None)
    }

    /// Fecha de una foto que se importa: como [`Taken::of_file`], más el
    /// `.json` de Takeout que tenga al lado.
    pub(crate) fn of_import(path: &Path, sidecars: &mut Sidecars) -> Self {
        Self::find(path, |p| sidecars.taken_at(p))
    }

    fn find(path: &Path, takeout: impl FnOnce(&Path) -> Option<i64>) -> Self {
        if let Some(t) = mnemos_imaging::capture_time(path) {
            return Self::new(When::Local(t), Source::Exif);
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default();
        let from_name = date_in_name(&name);
        match from_name {
            Some((when, true)) => Self::new(when, Source::Name),
            _ => match takeout(path) {
                Some(secs) => Self::new(When::Unix(secs), Source::Takeout),
                None => from_name.map_or(Self::NONE, |(when, _)| Self::new(when, Source::NameDay)),
            },
        }
    }

    /// Valores de `taken_at` para [`taken_at_sql`]: segundos exactos y hora
    /// local (`AAAA-MM-DD hh:mm:ss`).
    pub(crate) fn sql_params(&self) -> (Option<i64>, Option<String>) {
        match self.when {
            Some(When::Unix(secs)) => (Some(secs), None),
            Some(When::Local(t)) => (
                None,
                Some(format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    t.year, t.month, t.day, t.hour, t.minute, t.second
                )),
            ),
            None => (None, None),
        }
    }

    /// Año y mes para la carpeta `Originales/AAAA/MM/`, si hay fecha.
    pub(crate) fn year_month(&self) -> Option<(i64, u32)> {
        match self.when? {
            When::Unix(secs) => Some(year_month(secs)),
            When::Local(t) => Some((i64::from(t.year), u32::from(t.month))),
        }
    }
}

/// Segundos Unix de una hora local leída como si fuera UTC.
fn local_secs(t: &CaptureTime) -> i64 {
    days_from_civil(i64::from(t.year), u32::from(t.month), u32::from(t.day)) * 86_400
        + i64::from(t.hour) * 3_600
        + i64::from(t.minute) * 60
        + i64::from(t.second)
}

/// Busca una fecha en el nombre de un archivo (sin extensión). Devuelve la
/// fecha y si trae también la hora.
///
/// Reconoce `AAAAMMDD`, `AAAA-MM-DD` (con `-`, `_`, `.` o espacio),
/// `DD-MM-AAAA`, `AAAAMMDDhhmmss` y marcas Unix de 13 cifras (milisegundos),
/// con la hora detrás como `hhmmss` o `hh-mm-ss`.
pub(crate) fn date_in_name(name: &str) -> Option<(When, bool)> {
    let runs = digit_runs(name);
    for i in 0..runs.len() {
        let run = |k: usize| runs.get(k).map(|&(s, e)| &name[s..e]);
        let sep = |k: usize| -> bool {
            // Un solo separador entre la cifra k-1 y la k.
            match (runs.get(k - 1), runs.get(k)) {
                (Some(&(_, e)), Some(&(s, _))) => {
                    s == e + 1 && matches!(name.as_bytes()[e], b'-' | b'_' | b'.' | b' ')
                }
                _ => false,
            }
        };
        let current = run(i)?;
        let (date, next) = match current.len() {
            14 => {
                let date = parse_date(&current[..4], &current[4..6], &current[6..8]);
                let time = date.and_then(|d| {
                    with_time(d, &current[8..10], &current[10..12], &current[12..14])
                });
                if let Some(t) = time {
                    return Some((When::Local(t), true));
                }
                (date, i + 1)
            }
            13 if !name[..runs[i].0].ends_with('-') => {
                let secs = current.parse::<i64>().ok()? / 1000;
                // De 2000 a 2100.
                if (946_684_800..4_102_444_800).contains(&secs) {
                    return Some((When::Unix(secs), true));
                }
                continue;
            }
            8 => (
                parse_date(&current[..4], &current[4..6], &current[6..8]),
                i + 1,
            ),
            4 if run(i + 1).map(str::len) == Some(2)
                && run(i + 2).map(str::len) == Some(2)
                && sep(i + 1)
                && sep(i + 2) =>
            {
                (parse_date(current, run(i + 1)?, run(i + 2)?), i + 3)
            }
            2 if run(i + 1).map(str::len) == Some(2)
                && run(i + 2).map(str::len) == Some(4)
                && dash(name, &runs, i + 1)
                && dash(name, &runs, i + 2) =>
            {
                (parse_date(run(i + 2)?, run(i + 1)?, current), i + 3)
            }
            _ => continue,
        };
        let Some(date) = date else { continue };
        return Some(
            match time_after(name, &runs, runs[next - 1].1, next, date) {
                Some(t) => (When::Local(t), true),
                None => (When::Local(date), false),
            },
        );
    }
    None
}

/// Indica si entre la cifra k-1 y la k hay justo un guion.
fn dash(name: &str, runs: &[(usize, usize)], k: usize) -> bool {
    match (runs.get(k - 1), runs.get(k)) {
        (Some(&(_, e)), Some(&(s, _))) => s == e + 1 && name.as_bytes()[e] == b'-',
        _ => false,
    }
}

/// Posiciones (inicio, fin) de cada grupo de cifras ASCII.
fn digit_runs(name: &str) -> Vec<(usize, usize)> {
    let bytes = name.as_bytes();
    let mut runs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            runs.push((start, i));
        } else {
            i += 1;
        }
    }
    runs
}

/// Hora detrás de la fecha: `hhmmss` (con milisegundos o sin ellos) o
/// `hh-mm-ss`, separada de la fecha por un texto corto (`_`, ` at `, ` a las `).
fn time_after(
    name: &str,
    runs: &[(usize, usize)],
    date_end: usize,
    k: usize,
    date: CaptureTime,
) -> Option<CaptureTime> {
    let &(start, end) = runs.get(k)?;
    let gap = &name[date_end..start];
    if gap.is_empty() || gap.len() > 8 {
        return None;
    }
    let first = &name[start..end];
    if (6..=9).contains(&first.len()) {
        return with_time(date, &first[..2], &first[2..4], &first[4..6]);
    }
    let parts: Vec<&str> = runs
        .get(k..k + 3)?
        .iter()
        .map(|&(s, e)| &name[s..e])
        .collect();
    let joined = runs[k..k + 3].windows(2).all(|w| {
        w[1].0 == w[0].1 + 1 && matches!(name.as_bytes()[w[0].1], b'-' | b'_' | b'.' | b':' | b' ')
    });
    if joined && parts.iter().all(|p| p.len() == 2) {
        return with_time(date, parts[0], parts[1], parts[2]);
    }
    None
}

fn parse_date(year: &str, month: &str, day: &str) -> Option<CaptureTime> {
    let t = CaptureTime {
        year: year.parse().ok()?,
        month: month.parse().ok()?,
        day: day.parse().ok()?,
        hour: 0,
        minute: 0,
        second: 0,
        offset_minutes: None,
    };
    ((1990..=2100).contains(&t.year) && t.is_valid()).then_some(t)
}

fn with_time(date: CaptureTime, hour: &str, minute: &str, second: &str) -> Option<CaptureTime> {
    let t = CaptureTime {
        hour: hour.parse().ok()?,
        minute: minute.parse().ok()?,
        second: second.parse().ok()?,
        ..date
    };
    t.is_valid().then_some(t)
}

/// Los `.json` de Google Takeout de cada carpeta del origen, leídos una sola
/// vez por carpeta.
#[derive(Debug, Default)]
pub(crate) struct Sidecars {
    dirs: HashMap<PathBuf, Vec<String>>,
}

/// Sufijos que Google Fotos añade a las copias editadas, que no tienen `.json`
/// propio: se usa el del original.
const EDITED_SUFFIXES: &[&str] = &["-editada", "-editado", "-edited"];

const SUPPLEMENTAL: &str = ".supplemental-metadata";

impl Sidecars {
    /// `photoTakenTime` del `.json` de la foto, en segundos Unix.
    pub(crate) fn taken_at(&mut self, photo: &Path) -> Option<i64> {
        let dir = photo.parent()?;
        let name = photo.file_name()?.to_str()?;
        let jsons = self.dirs.entry(dir.to_owned()).or_insert_with(|| {
            fs::read_dir(dir)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter(|n| n.to_ascii_lowercase().ends_with(".json"))
                .collect()
        });
        let json = sidecar_for(name, jsons)?;
        read_photo_taken_time(&dir.join(json))
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TakeoutJson {
    photo_taken_time: Option<TakeoutTime>,
}

#[derive(Deserialize)]
struct TakeoutTime {
    timestamp: String,
}

fn read_photo_taken_time(path: &Path) -> Option<i64> {
    // Los .json de Takeout ocupan unos cientos de bytes; uno enorme no es suyo.
    if fs::metadata(path).ok()?.len() > 1 << 20 {
        return None;
    }
    let json: TakeoutJson = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    let secs: i64 = json.photo_taken_time?.timestamp.parse().ok()?;
    (secs > 0).then_some(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(name: &str) -> Option<(String, bool)> {
        date_in_name(name).map(|(when, with_time)| {
            let text = match when {
                When::Local(t) => format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    t.year, t.month, t.day, t.hour, t.minute, t.second
                ),
                When::Unix(secs) => format!("@{secs}"),
            };
            (text, with_time)
        })
    }

    #[test]
    fn reconoce_las_fechas_de_los_nombres_habituales() {
        let casos = [
            ("IMG_20190512_143000", "2019-05-12 14:30:00", true),
            ("IMG_20190222_144841_480", "2019-02-22 14:48:41", true),
            ("PXL_20230101_235959123", "2023-01-01 23:59:59", true),
            ("IMG20220110212038(1)", "2022-01-10 21:20:38", true),
            (
                "IMG20220107222220_11-ANIMATION",
                "2022-01-07 22:22:20",
                true,
            ),
            ("20210830_115519-editada", "2021-08-30 11:55:19", true),
            ("Rocío 20180317_223639", "2018-03-17 22:36:39", true),
            ("download_20170425_184123", "2017-04-25 18:41:23", true),
            ("Screenshot_20190813-211846", "2019-08-13 21:18:46", true),
            (
                "Screenshot_2018-09-11-22-06-14",
                "2018-09-11 22:06:14",
                true,
            ),
            (
                "Screenshot_2024-08-04-08-57-24-67",
                "2024-08-04 08:57:24",
                true,
            ),
            (
                "Record_2022-05-22-13-35-29.mp4_snapshot_00.16.563",
                "2022-05-22 13:35:29",
                true,
            ),
            (
                "WhatsApp Image 2018-07-24 at 20.51.39",
                "2018-07-24 20:51:39",
                true,
            ),
            (
                "WhatsApp Image 2018-07-24 at 20.51.44 (1)",
                "2018-07-24 20:51:44",
                true,
            ),
            (
                "Captura de pantalla 2019-05-12 a las 14.30.00",
                "2019-05-12 14:30:00",
                true,
            ),
            ("photo_2023-10-19_11-30-02", "2023-10-19 11:30:02", true),
            ("photo_57@03-01-2023_21-18-03", "2023-01-03 21:18:03", true),
            ("IMG-20181215-WA0003", "2018-12-15 00:00:00", false),
            ("IMG-20180818-WA0004-editada", "2018-08-18 00:00:00", false),
        ];
        for (name, fecha, con_hora) in casos {
            assert_eq!(local(name), Some((fecha.to_owned(), con_hora)), "{name}");
        }
    }

    #[test]
    fn reconoce_las_marcas_unix_en_milisegundos() {
        assert_eq!(
            local("1586252852190_15192911"),
            Some(("@1586252852".into(), true))
        );
        assert_eq!(
            local("FB_IMG_1557664200123"),
            Some(("@1557664200".into(), true))
        );
    }

    #[test]
    fn no_confunde_otros_numeros_con_fechas() {
        for name in [
            "DSC01234",
            "12345678",
            "-5987933178498762688_121",
            "70694167_130224808308811_1342624452756796962_n_2805438903024265",
            "Untitled-Made with FlexClip (7)",
            "Splatoon_2_Octolings_[1440x2560]",
            "IMG_20191345_120000",
            "unknown",
        ] {
            assert_eq!(local(name), None, "{name}");
        }
    }

    #[test]
    fn una_hora_imposible_deja_solo_el_dia() {
        assert_eq!(
            local("IMG_20190512_256100"),
            Some(("2019-05-12 00:00:00".into(), false))
        );
    }

    #[test]
    fn el_desfase_del_exif_da_la_hora_exacta() {
        let t = CaptureTime {
            year: 2019,
            month: 5,
            day: 12,
            hour: 14,
            minute: 30,
            second: 0,
            offset_minutes: Some(120),
        };
        // 2019-05-12 12:30:00 UTC.
        assert_eq!(
            Taken::new(When::Local(t), Source::Exif).when,
            Some(When::Unix(1_557_664_200))
        );
    }

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

    #[test]
    fn lee_photo_taken_time_e_ignora_json_raros() {
        let dir = tempfile::tempdir().unwrap();
        let foto = dir.path().join("IMG_1.jpg");
        let json = dir.path().join("IMG_1.jpg.supplemental-metadata.json");
        fs::write(
            &json,
            r#"{"title":"IMG_1.jpg","creationTime":{"timestamp":"1715871078"},
               "photoTakenTime":{"timestamp":"1679579100","formatted":"23 mar 2023"}}"#,
        )
        .unwrap();
        assert_eq!(Sidecars::default().taken_at(&foto), Some(1_679_579_100));

        for raro in [
            "{}",
            "no es json",
            r#"{"photoTakenTime":{"timestamp":"0"}}"#,
        ] {
            fs::write(&json, raro).unwrap();
            assert_eq!(Sidecars::default().taken_at(&foto), None, "{raro}");
        }
    }
}
