//! Fecha de captura de una foto (spec 004): EXIF, nombre del archivo o el
//! `.json` que deja Google Takeout.

use std::path::Path;

use mnemos_imaging::CaptureTime;
use rusqlite::{Connection, params};

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
    /// Fecha de grabación guardada en el contenedor de un vídeo (spec 019).
    /// Vale lo mismo que el EXIF de una foto.
    Video,
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
            Source::Video => "video",
        }
    }

    /// Expresión SQL con el orden de la fuente guardada en `column`: las que
    /// no conoce (o `NULL`, sin buscar todavía) valen lo mismo que `ninguna`.
    /// La fecha a mano (spec 011) está por encima de todas.
    pub(crate) fn rank_sql(column: &str) -> String {
        format!(
            "CASE {column} WHEN '{MANUAL}' THEN 5
                WHEN 'exif' THEN 4 WHEN 'video' THEN 4 WHEN 'nombre' THEN 3 WHEN 'takeout' THEN 2
                WHEN 'nombre_dia' THEN 1 ELSE 0 END"
        )
    }

    pub(crate) fn rank(self) -> i64 {
        match self {
            Source::Video => Source::Exif as i64,
            other => other as i64,
        }
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
    /// Hora de pared que marcaba la cámara (segundos leídos como UTC) y su
    /// desfase en minutos, si la fuente la da (spec 022). Sin ella, la hora
    /// local se saca de `when` con la zona del equipo.
    pub wall: Option<(i64, Option<i16>)>,
}

/// Parámetros de una fecha para [`taken_at_sql`] y [`taken_local_sql`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TakenParams {
    /// Segundos Unix exactos.
    pub exact: Option<i64>,
    /// Hora local sin zona (`AAAA-MM-DD hh:mm:ss`), que se pasa a UTC con la
    /// zona del equipo.
    pub local: Option<String>,
    /// Hora de pared para `taken_local`.
    pub wall: Option<i64>,
    /// Desfase para `taken_offset`.
    pub offset: Option<i16>,
}

/// Expresión SQL del valor de `taken_at` a partir de dos parámetros: segundos
/// exactos y hora local (ver [`Taken::sql_params`]).
pub(crate) fn taken_at_sql(exact: usize, local: usize) -> String {
    format!("coalesce(?{exact}, unixepoch(?{local}, 'utc'))")
}

/// Expresión SQL del valor de `taken_local` (spec 022): la hora de pared del
/// parámetro `wall` o, si no la hay, el instante `at` (o `fallback` si `at`
/// es NULL) pasado a hora local con la zona del equipo. `fallback` es una
/// expresión SQL: `modified_at` en un UPDATE, el parámetro de la fecha de
/// modificación en un INSERT o `NULL` para la fecha automática de reserva.
pub(crate) fn taken_local_sql(wall: usize, at: &str, fallback: &str) -> String {
    format!("coalesce(?{wall}, unixepoch(coalesce({at}, {fallback}), 'unixepoch', 'localtime'))")
}

/// `taken_source` de una fecha puesta a mano (spec 011). No es una [`Source`]:
/// ningún proceso automático la encuentra ni la reemplaza.
pub(crate) const MANUAL: &str = "manual";

/// Guarda la fecha de captura de la foto `rel_path`. Si tiene una fecha a mano
/// (spec 011), la deja y guarda esta como la automática de reserva.
pub(crate) fn set_taken(conn: &Connection, rel_path: &str, taken: &Taken) -> Result<()> {
    let p = taken.sql_params();
    let args = params![
        rel_path,
        p.exact,
        p.local,
        taken.source.as_str(),
        p.wall,
        p.offset
    ];
    let at = taken_at_sql(2, 3);
    conn.execute(
        &format!(
            "UPDATE photos SET taken_at = {at}, taken_source = ?4,
                 taken_local = {local}, taken_offset = ?6
             WHERE rel_path = ?1 AND taken_source IS NOT '{MANUAL}'",
            local = taken_local_sql(5, &at, "modified_at")
        ),
        args,
    )?;
    conn.execute(
        &format!(
            "UPDATE photos SET taken_auto_at = {at}, taken_auto_source = ?4,
                 taken_auto_local = {local}, taken_auto_offset = ?6
             WHERE rel_path = ?1 AND taken_source = '{MANUAL}'",
            local = taken_local_sql(5, &at, "NULL")
        ),
        args,
    )?;
    Ok(())
}

/// Afina la hora local de una foto ya registrada (spec 022) con lo que dice
/// su original: si `taken` sale de la misma fuente que la fecha guardada y
/// trae la hora de la cámara, guarda esa hora y su desfase. No cambia la
/// fecha (`taken_at`) ni su fuente; con fecha a mano, afina la automática de
/// reserva. Una fecha del Takeout o del `creation_time` de un vídeo no trae
/// hora de la cámara: se queda con la de reserva.
pub(crate) fn refine_local(conn: &Connection, rel_path: &str, taken: &Taken) -> Result<()> {
    let Some((wall, offset)) = taken.wall else {
        return Ok(());
    };
    let source = taken.source.as_str();
    conn.execute(
        "UPDATE photos SET taken_local = ?2, taken_offset = ?3
         WHERE rel_path = ?1 AND taken_source = ?4",
        params![rel_path, wall, offset, source],
    )?;
    conn.execute(
        &format!(
            "UPDATE photos SET taken_auto_local = ?2, taken_auto_offset = ?3
             WHERE rel_path = ?1 AND taken_source = '{MANUAL}' AND taken_auto_source = ?4"
        ),
        params![rel_path, wall, offset, source],
    )?;
    Ok(())
}

/// Mejora la fecha de las fotos con esta huella si `taken` sale de una fuente
/// más fiable que la que tienen. En las que tienen fecha a mano mejora la
/// automática de reserva. Devuelve cuántas cambió.
pub(crate) fn improve_taken(conn: &Connection, hash: &str, taken: &Taken) -> Result<usize> {
    let p = taken.sql_params();
    let args = params![
        hash,
        p.exact,
        p.local,
        taken.source.as_str(),
        taken.source.rank(),
        p.wall,
        p.offset
    ];
    let at = taken_at_sql(2, 3);
    let mut changed = conn.execute(
        &format!(
            "UPDATE photos SET taken_at = {at}, taken_source = ?4,
                 taken_local = {local}, taken_offset = ?7
             WHERE content_hash = ?1 AND {rank} < ?5",
            local = taken_local_sql(6, &at, "modified_at"),
            rank = Source::rank_sql("taken_source")
        ),
        args,
    )?;
    changed += conn.execute(
        &format!(
            "UPDATE photos SET taken_auto_at = {at}, taken_auto_source = ?4,
                 taken_auto_local = {local}, taken_auto_offset = ?7
             WHERE content_hash = ?1 AND taken_source = '{MANUAL}' AND {rank} < ?5",
            local = taken_local_sql(6, &at, "NULL"),
            rank = Source::rank_sql("taken_auto_source")
        ),
        args,
    )?;
    Ok(changed)
}

/// Hora local escrita a mano (`AAAA-MM-DD hh:mm[:ss]`), validada como una
/// fecha del EXIF (de 1900 a 2200). Devuelve el texto normalizado con segundos.
pub(crate) fn parse_manual(text: &str) -> Option<String> {
    let (date, time) = text.trim().split_once([' ', 'T'])?;
    let date: Vec<&str> = date.split('-').collect();
    let mut time: Vec<&str> = time.split(':').collect();
    if time.len() == 2 {
        time.push("00");
    }
    let lens = [4, 2, 2, 2, 2, 2];
    let parts: Vec<&str> = date.into_iter().chain(time).collect();
    if parts.len() != lens.len()
        || parts
            .iter()
            .zip(lens)
            .any(|(p, n)| p.len() != n || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let num = |i: usize| parts[i].parse::<u8>().ok();
    let t = CaptureTime {
        year: parts[0].parse().ok()?,
        month: num(1)?,
        day: num(2)?,
        hour: num(3)?,
        minute: num(4)?,
        second: num(5)?,
        offset_minutes: None,
    };
    t.is_valid().then(|| {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            t.year, t.month, t.day, t.hour, t.minute, t.second
        )
    })
}

impl Taken {
    pub(crate) const NONE: Taken = Taken {
        when: None,
        source: Source::None,
        wall: None,
    };

    fn new(when: When, source: Source) -> Self {
        let (when, wall) = match when {
            When::Local(t) => {
                let wall = local_secs(&t);
                match t.offset_minutes {
                    // `creation_time` de un vídeo es UTC (desfase 0), no la
                    // hora de la cámara: se trata como un instante y la hora
                    // local sale de la zona del equipo, como en el Takeout.
                    Some(0) if source == Source::Video => (When::Unix(wall), None),
                    // Los desfases reales van de −12:00 a +14:00.
                    Some(offset) if (-14 * 60..=14 * 60).contains(&offset) => (
                        When::Unix(wall - i64::from(offset) * 60),
                        Some((wall, Some(offset))),
                    ),
                    Some(offset) => (When::Unix(wall - i64::from(offset) * 60), None),
                    None => (When::Local(t), Some((wall, None))),
                }
            }
            unix => (unix, None),
        };
        Taken {
            when: Some(when),
            source,
            wall,
        }
    }

    /// Fecha de una foto que ya está en la biblioteca: EXIF o nombre.
    pub(crate) fn of_file(path: &Path) -> Self {
        Self::find(path, None)
    }

    /// Fecha de una foto que se importa: como [`Taken::of_file`], más el
    /// `photoTakenTime` del `.json` de Takeout que tenga al lado.
    pub(crate) fn of_import(path: &Path, takeout: Option<i64>) -> Self {
        Self::find(path, takeout)
    }

    fn find(path: &Path, takeout: Option<i64>) -> Self {
        if mnemos_imaging::is_video(path) {
            if let Some(t) = mnemos_imaging::video_info(path).and_then(|v| v.capture_time) {
                return Self::new(When::Local(t), Source::Video);
            }
        } else if let Some(t) = mnemos_imaging::capture_time(path) {
            return Self::new(When::Local(t), Source::Exif);
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default();
        let from_name = date_in_name(&name);
        match from_name {
            Some((when, true)) => Self::new(when, Source::Name),
            _ => match takeout {
                Some(secs) => Self::new(When::Unix(secs), Source::Takeout),
                None => from_name.map_or(Self::NONE, |(when, _)| Self::new(when, Source::NameDay)),
            },
        }
    }

    /// Parámetros para [`taken_at_sql`] y [`taken_local_sql`].
    pub(crate) fn sql_params(&self) -> TakenParams {
        let (exact, local) = match self.when {
            Some(When::Unix(secs)) => (Some(secs), None),
            Some(When::Local(t)) => (
                None,
                Some(format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    t.year, t.month, t.day, t.hour, t.minute, t.second
                )),
            ),
            None => (None, None),
        };
        TakenParams {
            exact,
            local,
            wall: self.wall.map(|(wall, _)| wall),
            offset: self.wall.and_then(|(_, offset)| offset),
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
    fn valida_la_fecha_escrita_a_mano() {
        assert_eq!(
            parse_manual("2019-05-12 14:30").as_deref(),
            Some("2019-05-12 14:30:00")
        );
        assert_eq!(
            parse_manual(" 1905-02-28T08:09:10 ").as_deref(),
            Some("1905-02-28 08:09:10")
        );
        for mala in [
            "2019-02-30 10:00",
            "1899-12-31 23:59",
            "2201-01-01 00:00",
            "2019-05-12 24:00",
            "2019-05-12",
            "2019-5-12 14:30",
            "2019-05-12 +1:30",
            "12/05/2019 14:30",
            "",
        ] {
            assert_eq!(parse_manual(mala), None, "{mala}");
        }
    }

    fn exif(hour: u8, offset: Option<i16>) -> CaptureTime {
        CaptureTime {
            year: 2024,
            month: 7,
            day: 14,
            hour,
            minute: 30,
            second: 0,
            offset_minutes: offset,
        }
    }

    /// `2024-07-14 23:30:00` leído como UTC.
    const PARED: i64 = 1_720_999_800;

    #[test]
    fn la_hora_de_pared_sale_del_exif_con_o_sin_desfase() {
        let con = Taken::new(When::Local(exif(23, Some(-240))), Source::Exif);
        assert_eq!(con.wall, Some((PARED, Some(-240))));
        // 23:30 en UTC−4 son las 03:30 UTC del día siguiente.
        assert_eq!(con.when, Some(When::Unix(PARED + 4 * 3600)));

        let sin = Taken::new(When::Local(exif(23, None)), Source::Name);
        assert_eq!(sin.wall, Some((PARED, None)));
        assert_eq!(sin.when, Some(When::Local(exif(23, None))));
        let p = sin.sql_params();
        assert_eq!((p.wall, p.offset), (Some(PARED), None));
        assert_eq!(p.local.as_deref(), Some("2024-07-14 23:30:00"));
    }

    #[test]
    fn los_instantes_sin_hora_de_camara_no_traen_hora_de_pared() {
        // `creation_time` de un vídeo: UTC, no la hora de la cámara.
        let video = Taken::new(When::Local(exif(23, Some(0))), Source::Video);
        assert_eq!(video.wall, None);
        assert_eq!(video.when, Some(When::Unix(PARED)));
        // La fecha de los vídeos de Apple sí trae su desfase.
        let apple = Taken::new(When::Local(exif(23, Some(120))), Source::Video);
        assert_eq!(apple.wall, Some((PARED, Some(120))));
        // Takeout y marcas de tiempo en el nombre.
        assert_eq!(Taken::new(When::Unix(PARED), Source::Takeout).wall, None);
        // Un desfase imposible no se guarda como desfase.
        let raro = Taken::new(When::Local(exif(23, Some(15 * 60))), Source::Exif);
        assert_eq!(raro.wall, None);
    }

    fn biblioteca() -> (tempfile::TempDir, crate::Library, i64) {
        let dir = tempfile::tempdir().unwrap();
        let lib = crate::Library::create(dir.path()).unwrap();
        lib.connection()
            .execute(
                "INSERT INTO photos (rel_path, size_bytes, modified_at, imported_at)
                 VALUES ('Originales/a.jpg', 1, 1000, 0)",
                [],
            )
            .unwrap();
        let id = lib.connection().last_insert_rowid();
        (dir, lib, id)
    }

    fn hora_local(lib: &crate::Library, id: i64) -> (Option<i64>, Option<i64>, Option<i64>) {
        lib.connection()
            .query_row(
                "SELECT taken_local, taken_offset, taken_at FROM photos WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap()
    }

    #[test]
    fn guarda_la_hora_local_y_el_desfase_en_cualquier_zona() {
        let (_dir, lib, id) = biblioteca();
        let conn = lib.connection();
        let con = Taken::new(When::Local(exif(23, Some(-240))), Source::Exif);
        set_taken(conn, "Originales/a.jpg", &con).unwrap();
        assert_eq!(
            hora_local(&lib, id),
            (Some(PARED), Some(-240), Some(PARED + 4 * 3600))
        );

        // Sin hora de pared, la de reserva sale del instante con la zona del
        // equipo, y el desfase se borra.
        let takeout = Taken::new(When::Unix(PARED), Source::Takeout);
        set_taken(conn, "Originales/a.jpg", &takeout).unwrap();
        let esperada: i64 = conn
            .query_row(
                "SELECT unixepoch(?1, 'unixepoch', 'localtime')",
                [PARED],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hora_local(&lib, id), (Some(esperada), None, Some(PARED)));

        // Sin fecha de captura, la de modificación.
        set_taken(conn, "Originales/a.jpg", &Taken::NONE).unwrap();
        let reserva: i64 = conn
            .query_row(
                "SELECT unixepoch(1000, 'unixepoch', 'localtime')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hora_local(&lib, id), (Some(reserva), None, None));
    }

    #[test]
    fn la_fecha_a_mano_guarda_y_devuelve_la_hora_local_automatica() {
        let (_dir, lib, id) = biblioteca();
        let con = Taken::new(When::Local(exif(23, Some(-240))), Source::Exif);
        set_taken(lib.connection(), "Originales/a.jpg", &con).unwrap();

        assert!(lib.set_manual_taken(id, "2020-01-02 10:00").unwrap());
        let (local, offset, _) = hora_local(&lib, id);
        assert_eq!((local, offset), (Some(1_577_959_200), None));
        // Mientras hay fecha a mano, reindexar mejora la automática.
        set_taken(lib.connection(), "Originales/a.jpg", &con).unwrap();
        let auto: (Option<i64>, Option<i64>) = lib
            .connection()
            .query_row(
                "SELECT taken_auto_local, taken_auto_offset FROM photos WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(auto, (Some(PARED), Some(-240)));

        assert!(lib.clear_manual_taken(id).unwrap());
        assert_eq!(
            hora_local(&lib, id),
            (Some(PARED), Some(-240), Some(PARED + 4 * 3600))
        );
    }

    #[test]
    fn otras_escrituras_reciben_la_hora_local_de_reserva() {
        let (_dir, lib, id) = biblioteca();
        let conn = lib.connection();
        let local = |secs: i64| -> i64 {
            conn.query_row(
                "SELECT unixepoch(?1, 'unixepoch', 'localtime')",
                [secs],
                |r| r.get(0),
            )
            .unwrap()
        };
        // Una fila sin hora local la recibe de la fecha de modificación…
        assert_eq!(hora_local(&lib, id).0, Some(local(1000)));
        // …y de `taken_at` cuando alguien la cambia sin dar la hora local.
        conn.execute("UPDATE photos SET taken_at = ?1 WHERE id = ?2", [PARED, id])
            .unwrap();
        assert_eq!(
            hora_local(&lib, id),
            (Some(local(PARED)), None, Some(PARED))
        );
        // Cambiar la fecha de modificación no la toca si hay fecha de captura.
        conn.execute("UPDATE photos SET modified_at = 5 WHERE id = ?1", [id])
            .unwrap();
        assert_eq!(hora_local(&lib, id).0, Some(local(PARED)));
    }
}
