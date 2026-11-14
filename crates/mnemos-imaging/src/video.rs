//! Datos de los vídeos (spec 019), leídos del contenedor ISO (MP4, MOV, 3GP)
//! sin descodificar nada.
//!
//! Solo se leen las cajas de `moov`; los datos de la imagen y el sonido
//! (`mdat`) se saltan con `seek`, así que un vídeo de varios GB se lee en
//! milisegundos.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::capture::CaptureTime;

/// Extensiones de vídeo que MNEMOS reconoce, en minúsculas.
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mov", "3gp", "3g2", "webm", "mkv", "avi", "mts", "m2ts", "wmv",
];

/// Extensiones con contenedor ISO, el único que se sabe leer.
const ISO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mov", "3gp", "3g2"];

/// Indica si el archivo es un vídeo, según su extensión.
pub fn is_video(path: &Path) -> bool {
    crate::has_extension(path, VIDEO_EXTENSIONS)
}

/// Lo que se sabe de un vídeo sin reproducirlo.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct VideoInfo {
    pub duration_ms: Option<u64>,
    /// Ancho y alto con el giro ya aplicado, como se ve.
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Giro en grados en el sentido de las agujas del reloj: 0, 90, 180 o 270.
    pub rotation: u16,
    /// Fecha de grabación: la de Apple (con su desfase horario) o la de
    /// creación del contenedor (en UTC).
    #[serde(skip)]
    pub capture_time: Option<CaptureTime>,
    pub fps: Option<f64>,
    /// Códecs con su nombre común: «H.264», «HEVC», «AAC»…
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
    /// Latitud y longitud en grados decimales (sur y oeste en negativo).
    pub gps: Option<(f64, f64)>,
}

/// Datos del vídeo leídos de su contenedor.
///
/// Devuelve `None` si la extensión no es de un contenedor ISO o el archivo no
/// tiene una caja `moov` legible.
pub fn video_info(path: &Path) -> Option<VideoInfo> {
    if !crate::has_extension(path, ISO_EXTENSIONS) {
        return None;
    }
    let file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    read_info(&mut BufReader::new(file), len)
}

/// Segundos entre 1904-01-01 (época de QuickTime) y 1970-01-01.
const EPOCH_1904: i64 = 2_082_844_800;

/// Tope para las cajas que se leen enteras. `moov` lleva tablas por muestra
/// que en un vídeo largo ocupan varios MB; solo se leen las pequeñas.
const MAX_LEAF: u64 = 1 << 20;

/// Tope de cajas recorridas, por si el archivo está dañado.
const MAX_BOXES: usize = 10_000;

#[derive(Default)]
struct Track {
    handler: [u8; 4],
    width: u32,
    height: u32,
    rotation: u16,
    timescale: u32,
    duration: u64,
    samples: u64,
    codec: Option<[u8; 4]>,
    sample_width: u32,
    sample_height: u32,
}

#[derive(Default)]
struct Parsed {
    timescale: u32,
    duration: u64,
    created: Option<i64>,
    tracks: Vec<Track>,
    make: Option<String>,
    model: Option<String>,
    location: Option<String>,
    apple_date: Option<String>,
}

pub(crate) fn read_info(r: &mut (impl Read + Seek), len: u64) -> Option<VideoInfo> {
    let mut p = Parsed::default();
    let mut budget = MAX_BOXES;
    let mut found = false;
    for b in boxes(r, 0, len, &mut budget) {
        if &b.kind == b"moov" {
            found = true;
            moov(r, &b, &mut p, &mut budget);
            break;
        }
    }
    found.then(|| p.into_info())
}

impl Parsed {
    fn into_info(self) -> VideoInfo {
        let video = self.tracks.iter().find(|t| &t.handler == b"vide");
        let audio = self.tracks.iter().find(|t| &t.handler == b"soun");
        let duration_ms = (self.timescale > 0 && self.duration > 0)
            .then(|| self.duration.saturating_mul(1000) / u64::from(self.timescale));
        let (mut width, mut height, rotation, fps) = match video {
            Some(t) => {
                let (w, h) = if t.width > 0 && t.height > 0 {
                    (t.width, t.height)
                } else {
                    (t.sample_width, t.sample_height)
                };
                let fps = (t.timescale > 0 && t.duration > 0 && t.samples > 0).then(|| {
                    let fps = t.samples as f64 * f64::from(t.timescale) / t.duration as f64;
                    (fps * 100.0).round() / 100.0
                });
                (w, h, t.rotation, fps)
            }
            None => (0, 0, 0, None),
        };
        if rotation % 180 == 90 {
            std::mem::swap(&mut width, &mut height);
        }
        let capture_time = self
            .apple_date
            .as_deref()
            .and_then(parse_iso8601)
            .or_else(|| self.created.and_then(utc_time));
        VideoInfo {
            duration_ms,
            width: (width > 0).then_some(width),
            height: (height > 0).then_some(height),
            rotation,
            capture_time,
            fps,
            video_codec: video.and_then(|t| t.codec).map(codec_name),
            audio_codec: audio.and_then(|t| t.codec).map(codec_name),
            make: self.make,
            model: self.model,
            gps: self.location.as_deref().and_then(parse_iso6709),
        }
    }
}

struct BoxHeader {
    kind: [u8; 4],
    /// Dónde empieza el contenido, tras la cabecera.
    start: u64,
    /// Dónde termina la caja.
    end: u64,
}

impl BoxHeader {
    fn size(&self) -> u64 {
        self.end - self.start
    }
}

/// Cajas hijas entre `start` y `end`.
fn boxes(r: &mut (impl Read + Seek), start: u64, end: u64, budget: &mut usize) -> Vec<BoxHeader> {
    let mut pos = start;
    let mut out = Vec::new();
    while pos + 8 <= end && *budget > 0 {
        *budget -= 1;
        let Some(h) = header(r, pos, end) else { break };
        pos = h.end;
        out.push(h);
    }
    out
}

fn header(r: &mut (impl Read + Seek), pos: u64, parent_end: u64) -> Option<BoxHeader> {
    r.seek(SeekFrom::Start(pos)).ok()?;
    let mut h = [0u8; 8];
    r.read_exact(&mut h).ok()?;
    let size = u64::from(u32::from_be_bytes([h[0], h[1], h[2], h[3]]));
    let kind = [h[4], h[5], h[6], h[7]];
    let (start, end) = match size {
        0 => (pos + 8, parent_end),
        1 => {
            let mut big = [0u8; 8];
            r.read_exact(&mut big).ok()?;
            (pos + 16, pos.checked_add(u64::from_be_bytes(big))?)
        }
        s if s < 8 => return None,
        s => (pos + 8, pos + s),
    };
    (start <= end && end <= parent_end).then_some(BoxHeader { kind, start, end })
}

/// Contenido entero de una caja pequeña.
fn read_box(r: &mut (impl Read + Seek), b: &BoxHeader) -> Option<Vec<u8>> {
    if b.size() > MAX_LEAF {
        return None;
    }
    r.seek(SeekFrom::Start(b.start)).ok()?;
    let mut buf = vec![0u8; usize::try_from(b.size()).ok()?];
    r.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn moov(r: &mut (impl Read + Seek), parent: &BoxHeader, p: &mut Parsed, budget: &mut usize) {
    for b in boxes(r, parent.start, parent.end, budget) {
        match &b.kind {
            b"mvhd" => {
                if let Some(d) = read_box(r, &b) {
                    mvhd(&d, p);
                }
            }
            b"trak" => {
                let mut t = Track::default();
                trak(r, &b, &mut t, budget);
                p.tracks.push(t);
            }
            b"udta" => udta(r, &b, p, budget),
            b"meta" => meta(r, &b, p, budget),
            _ => {}
        }
    }
}

fn mvhd(d: &[u8], p: &mut Parsed) {
    let Some(&version) = d.first() else { return };
    let (created, timescale, duration) = if version == 1 {
        (be64(d, 4), be32(d, 20), be64(d, 24))
    } else {
        (
            be32(d, 4).map(u64::from),
            be32(d, 12),
            be32(d, 16).map(u64::from),
        )
    };
    p.timescale = timescale.unwrap_or(0);
    p.duration = duration.unwrap_or(0);
    // 0 es «sin fecha» (1904); algunas cámaras guardan la época Unix.
    p.created = created
        .filter(|&c| c > 0)
        .and_then(|c| i64::try_from(c).ok())
        .map(|c| c - EPOCH_1904)
        .filter(|&c| c > 0);
}

fn trak(r: &mut (impl Read + Seek), parent: &BoxHeader, t: &mut Track, budget: &mut usize) {
    for b in boxes(r, parent.start, parent.end, budget) {
        match &b.kind {
            b"tkhd" => {
                if let Some(d) = read_box(r, &b) {
                    tkhd(&d, t);
                }
            }
            b"mdia" | b"minf" | b"stbl" => trak(r, &b, t, budget),
            // En MOV, `minf` lleva otro `hdlr` (el de los datos): vale el
            // primero, el de `mdia`.
            b"hdlr" if t.handler == [0; 4] => {
                if let Some(d) = read_box(r, &b)
                    && let Some(h) = d.get(8..12)
                {
                    t.handler.copy_from_slice(h);
                }
            }
            b"mdhd" => {
                if let Some(d) = read_box(r, &b) {
                    let (timescale, duration) = if d.first() == Some(&1) {
                        (be32(&d, 20), be64(&d, 24))
                    } else {
                        (be32(&d, 12), be32(&d, 16).map(u64::from))
                    };
                    t.timescale = timescale.unwrap_or(0);
                    t.duration = duration.unwrap_or(0);
                }
            }
            b"stsd" => {
                if let Some(d) = read_box(r, &b) {
                    stsd(&d, t);
                }
            }
            b"stts" => t.samples = stts(r, &b).unwrap_or(0),
            _ => {}
        }
    }
}

fn tkhd(d: &[u8], t: &mut Track) {
    // Tras la versión y los campos de fecha, id y duración, 8 reservados,
    // capa, grupo, volumen y 2 reservados (16 bytes) vienen la matriz y el
    // tamaño en 16.16.
    let matrix_at = if d.first() == Some(&1) {
        4 + 32 + 16
    } else {
        4 + 20 + 16
    };
    let m = |i: usize| be32(d, matrix_at + i * 4).map(|v| v as i32);
    if let (Some(a), Some(b), Some(c), Some(e)) = (m(0), m(1), m(3), m(4)) {
        t.rotation = match (a.signum(), b.signum(), c.signum(), e.signum()) {
            (0, 1, -1, 0) => 90,
            (-1, 0, 0, -1) => 180,
            (0, -1, 1, 0) => 270,
            _ => 0,
        };
    }
    t.width = be32(d, matrix_at + 36).map_or(0, |v| v >> 16);
    t.height = be32(d, matrix_at + 40).map_or(0, |v| v >> 16);
}

fn stsd(d: &[u8], t: &mut Track) {
    // Versión y entradas (8 bytes); la primera entrada: tamaño y formato.
    let Some(kind) = d.get(12..16) else { return };
    let mut codec = [0u8; 4];
    codec.copy_from_slice(kind);
    t.codec = Some(codec);
    // Entrada visual: 6 reservados, índice, 16 bytes y luego ancho y alto.
    t.sample_width = be16(d, 8 + 8 + 24).map_or(0, u32::from);
    t.sample_height = be16(d, 8 + 8 + 26).map_or(0, u32::from);
}

/// Número de muestras de la pista según `stts`.
fn stts(r: &mut (impl Read + Seek), b: &BoxHeader) -> Option<u64> {
    r.seek(SeekFrom::Start(b.start + 4)).ok()?;
    let mut n = [0u8; 4];
    r.read_exact(&mut n).ok()?;
    let entries = u64::from(u32::from_be_bytes(n));
    if entries.checked_mul(8)? + 8 > b.size() || entries > MAX_LEAF / 8 {
        return None;
    }
    let mut total = 0u64;
    let mut e = [0u8; 8];
    for _ in 0..entries {
        r.read_exact(&mut e).ok()?;
        total += u64::from(u32::from_be_bytes([e[0], e[1], e[2], e[3]]));
    }
    Some(total)
}

/// `udta` de QuickTime: `©mak`, `©mod` y `©xyz`, cada una con su longitud y
/// su idioma delante del texto.
fn udta(r: &mut (impl Read + Seek), parent: &BoxHeader, p: &mut Parsed, budget: &mut usize) {
    for b in boxes(r, parent.start, parent.end, budget) {
        let slot = match &b.kind {
            b"\xa9mak" => &mut p.make,
            b"\xa9mod" => &mut p.model,
            b"\xa9xyz" => &mut p.location,
            b"meta" => {
                meta(r, &b, p, budget);
                continue;
            }
            _ => continue,
        };
        if slot.is_none()
            && let Some(d) = read_box(r, &b)
            && let Some(len) = be16(&d, 0)
            && let Some(text) = d.get(4..4 + usize::from(len))
        {
            *slot = clean(text);
        }
    }
}

/// `meta` de Apple con `keys` e `ilst` (`com.apple.quicktime.make`…). En MP4
/// `meta` lleva versión delante; en QuickTime no.
fn meta(r: &mut (impl Read + Seek), parent: &BoxHeader, p: &mut Parsed, budget: &mut usize) {
    let full = header(r, parent.start, parent.end)
        .is_none_or(|h| !matches!(&h.kind, b"hdlr" | b"keys" | b"ilst"));
    let start = parent.start + if full { 4 } else { 0 };
    let children = boxes(r, start, parent.end, budget);
    let Some(keys) = children
        .iter()
        .find(|b| &b.kind == b"keys")
        .and_then(|b| read_box(r, b))
        .map(|d| parse_keys(&d))
    else {
        return;
    };
    let Some(ilst) = children.iter().find(|b| &b.kind == b"ilst") else {
        return;
    };
    for item in boxes(r, ilst.start, ilst.end, budget) {
        let index = u32::from_be_bytes(item.kind) as usize;
        let Some(key) = index.checked_sub(1).and_then(|i| keys.get(i)) else {
            continue;
        };
        let slot = match key.as_str() {
            "com.apple.quicktime.make" => &mut p.make,
            "com.apple.quicktime.model" => &mut p.model,
            "com.apple.quicktime.location.ISO6709" => &mut p.location,
            "com.apple.quicktime.creationdate" => &mut p.apple_date,
            _ => continue,
        };
        // `data`: tamaño, tipo, tipo de valor y país e idioma, luego el texto.
        if let Some(d) = read_box(r, &item)
            && d.get(4..8) == Some(b"data")
            && let Some(text) = d.get(16..)
            && let Some(text) = clean(text)
        {
            *slot = Some(text);
        }
    }
}

fn parse_keys(d: &[u8]) -> Vec<String> {
    let count = be32(d, 4).unwrap_or(0);
    let mut out = Vec::new();
    let mut pos = 8;
    for _ in 0..count {
        let Some(size) = be32(d, pos).map(|s| s as usize) else {
            break;
        };
        if size < 8 || pos + size > d.len() {
            break;
        }
        out.push(String::from_utf8_lossy(&d[pos + 8..pos + size]).into_owned());
        pos += size;
    }
    out
}

fn clean(text: &[u8]) -> Option<String> {
    let s = String::from_utf8_lossy(text);
    let s = s.trim_matches(|c: char| c.is_whitespace() || c == '\0');
    (!s.is_empty()).then(|| s.chars().take(200).collect())
}

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn be64(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Nombre común del códec a partir del formato de `stsd`.
fn codec_name(fourcc: [u8; 4]) -> String {
    let name = match &fourcc {
        b"avc1" | b"avc3" => "H.264",
        b"hvc1" | b"hev1" => "HEVC",
        b"av01" => "AV1",
        b"vp09" => "VP9",
        b"vp08" => "VP8",
        b"mp4v" => "MPEG-4",
        b"s263" | b"h263" => "H.263",
        b"jpeg" | b"mjpa" | b"mjpb" => "Motion JPEG",
        b"apch" | b"apcn" | b"apcs" | b"apco" | b"ap4h" | b"ap4x" => "ProRes",
        b"mp4a" => "AAC",
        b"samr" => "AMR",
        b"sawb" => "AMR-WB",
        b"ac-3" => "AC-3",
        b"ec-3" => "E-AC-3",
        b"Opus" => "Opus",
        b"alac" => "ALAC",
        b"lpcm" | b"sowt" | b"twos" | b"raw " | b"in24" | b"fl32" => "PCM",
        b".mp3" => "MP3",
        _ => return String::from_utf8_lossy(&fourcc).trim().to_owned(),
    };
    name.to_owned()
}

/// Fecha de Apple: `2019-05-12T14:30:00+0200` (también con `Z` o `+02:00`).
fn parse_iso8601(s: &str) -> Option<CaptureTime> {
    let b = s.as_bytes();
    let num = |from: usize, len: usize| -> Option<u16> {
        let part = s.get(from..from + len)?;
        part.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| part.parse().ok())?
    };
    if b.get(4) != Some(&b'-') || b.get(7) != Some(&b'-') || !matches!(b.get(10), Some(b'T' | b' '))
    {
        return None;
    }
    let mut t = CaptureTime {
        year: num(0, 4)?,
        month: u8::try_from(num(5, 2)?).ok()?,
        day: u8::try_from(num(8, 2)?).ok()?,
        hour: u8::try_from(num(11, 2)?).ok()?,
        minute: u8::try_from(num(14, 2)?).ok()?,
        second: u8::try_from(num(17, 2).unwrap_or(0)).ok()?,
        offset_minutes: None,
    };
    // El desfase va tras los segundos y sus posibles decimales.
    let rest = s.get(19..).unwrap_or("");
    let rest = rest.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    t.offset_minutes = match rest.as_bytes().first() {
        Some(b'Z') => Some(0),
        Some(&sign @ (b'+' | b'-')) => {
            let digits: String = rest[1..].chars().filter(char::is_ascii_digit).collect();
            let h: i16 = digits.get(0..2)?.parse().ok()?;
            let m: i16 = digits.get(2..4).map_or(Some(0), |m| m.parse().ok())?;
            let total = h * 60 + m;
            Some(if sign == b'-' { -total } else { total })
        }
        _ => None,
    };
    t.is_valid().then_some(t)
}

/// Segundos Unix a fecha en UTC.
fn utc_time(secs: i64) -> Option<CaptureTime> {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Algoritmo de Howard Hinnant (días desde 1970 a fecha civil).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let t = CaptureTime {
        year: u16::try_from(year).ok()?,
        month: u8::try_from(month).ok()?,
        day: u8::try_from(day).ok()?,
        hour: u8::try_from(rem / 3600).ok()?,
        minute: u8::try_from(rem % 3600 / 60).ok()?,
        second: u8::try_from(rem % 60).ok()?,
        offset_minutes: Some(0),
    };
    t.is_valid().then_some(t)
}

/// Ubicación ISO 6709: `+43.1953-004.8547/` o con altitud,
/// `+43.1953-004.8547+120.5/`.
fn parse_iso6709(s: &str) -> Option<(f64, f64)> {
    let s = s.trim().trim_end_matches('/');
    let mut parts = Vec::new();
    let mut start = 0;
    for (i, c) in s.char_indices().skip(1) {
        if c == '+' || c == '-' {
            parts.push(&s[start..i]);
            start = i;
        }
    }
    parts.push(&s[start..]);
    let lat: f64 = parts.first()?.parse().ok()?;
    let lon: f64 = parts.get(1)?.parse().ok()?;
    ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) && (lat, lon) != (0.0, 0.0))
        .then_some((lat, lon))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures");

    fn bx(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(kind);
        v.extend_from_slice(body);
        v
    }

    fn info(bytes: &[u8]) -> Option<VideoInfo> {
        read_info(&mut Cursor::new(bytes), bytes.len() as u64)
    }

    #[test]
    fn lee_un_mp4_girado() {
        let v = video_info(&Path::new(FIXTURES).join("video-girado.mp4")).unwrap();
        assert_eq!(v.duration_ms, Some(1000));
        assert_eq!(v.rotation, 90);
        // 32 × 18 guardado en horizontal: se ve en vertical.
        assert_eq!((v.width, v.height), (Some(18), Some(32)));
        assert_eq!(v.fps, Some(10.0));
        assert_eq!(v.video_codec.as_deref(), Some("H.264"));
        assert_eq!(v.audio_codec.as_deref(), Some("AAC"));
        let t = v.capture_time.unwrap();
        assert_eq!(
            (t.year, t.month, t.day, t.hour, t.minute),
            (2019, 5, 12, 14, 30)
        );
        assert_eq!(t.offset_minutes, Some(0));
    }

    #[test]
    fn lee_camara_y_ubicacion_de_un_mov() {
        let v = video_info(&Path::new(FIXTURES).join("video-camara.mov")).unwrap();
        assert_eq!(v.rotation, 0);
        assert_eq!((v.width, v.height), (Some(32), Some(18)));
        assert_eq!(v.make.as_deref(), Some("Pentax"));
        assert_eq!(v.model.as_deref(), Some("K-70"));
        assert_eq!(v.gps, Some((43.1953, -4.8547)));
        let t = v.capture_time.unwrap();
        assert_eq!(
            (t.year, t.month, t.day, t.hour, t.minute),
            (2021, 8, 3, 9, 15)
        );
    }

    #[test]
    fn la_fecha_de_apple_manda_sobre_la_del_contenedor() {
        let mut mvhd = vec![0u8; 100];
        mvhd[4..8].copy_from_slice(&((1_600_000_000 + EPOCH_1904) as u32).to_be_bytes());
        mvhd[12..16].copy_from_slice(&600u32.to_be_bytes());
        mvhd[16..20].copy_from_slice(&3000u32.to_be_bytes());
        let key = b"com.apple.quicktime.creationdate";
        let mut keys = vec![0, 0, 0, 0, 0, 0, 0, 1];
        keys.extend(bx(b"mdta", key));
        let mut data = vec![0, 0, 0, 1, 0, 0, 0, 0];
        data.extend_from_slice(b"2019-05-12T14:30:00+0200");
        let ilst = bx(b"ilst", &bx(&1u32.to_be_bytes(), &bx(b"data", &data)));
        let meta = bx(
            b"meta",
            &[bx(b"hdlr", &[0; 24]), bx(b"keys", &keys), ilst].concat(),
        );
        let file = [
            bx(b"ftyp", b"qt  "),
            bx(b"moov", &[bx(b"mvhd", &mvhd), meta].concat()),
        ]
        .concat();
        let v = info(&file).unwrap();
        assert_eq!(v.duration_ms, Some(5000));
        let t = v.capture_time.unwrap();
        assert_eq!((t.year, t.hour, t.offset_minutes), (2019, 14, Some(120)));
    }

    #[test]
    fn salta_mdat_aunque_vaya_antes_que_moov() {
        let mut mvhd = vec![0u8; 100];
        mvhd[12..16].copy_from_slice(&1000u32.to_be_bytes());
        mvhd[16..20].copy_from_slice(&2500u32.to_be_bytes());
        let file = [
            bx(b"ftyp", b"isom"),
            bx(b"mdat", &[7; 5000]),
            bx(b"moov", &bx(b"mvhd", &mvhd)),
        ]
        .concat();
        let v = info(&file).unwrap();
        assert_eq!(v.duration_ms, Some(2500));
        assert_eq!(v.capture_time, None, "0 es «sin fecha»");
    }

    #[test]
    fn archivos_rotos_o_sin_moov_no_fallan() {
        assert_eq!(info(b""), None);
        assert_eq!(info(&bx(b"ftyp", b"isom")), None);
        // Tamaño mayor que el archivo.
        let mut roto = bx(b"moov", &[0; 16]);
        roto[0..4].copy_from_slice(&1_000_000u32.to_be_bytes());
        assert_eq!(info(&roto), None);
        // Caja de tamaño imposible dentro de moov.
        let dentro = bx(b"moov", &[0, 0, 0, 3, b't', b'r', b'a', b'k']);
        assert_eq!(info(&dentro), Some(VideoInfo::default()));
    }

    #[test]
    fn la_camara_de_un_video_sale_del_contenedor() {
        let c = crate::camera_info(&Path::new(FIXTURES).join("video-camara.mov")).unwrap();
        assert_eq!(c.camera.as_deref(), Some("Pentax K-70"));
        assert_eq!(c.gps, Some((43.1953, -4.8547)));
        assert_eq!(
            crate::camera_info(&Path::new(FIXTURES).join("video-girado.mp4")),
            None
        );
    }

    #[test]
    fn reconoce_las_extensiones_de_video() {
        assert!(is_video(Path::new("VID_20190512.MP4")));
        assert!(is_video(Path::new("a/IMG_0001.mov")));
        assert!(!is_video(Path::new("foto.jpg")));
        assert_eq!(
            video_info(Path::new("clip.webm")),
            None,
            "solo contenedores ISO"
        );
    }

    #[test]
    fn interpreta_fechas_y_ubicaciones() {
        let t = parse_iso8601("2020-01-02T03:04:05Z").unwrap();
        assert_eq!(
            (t.year, t.month, t.day, t.second, t.offset_minutes),
            (2020, 1, 2, 5, Some(0))
        );
        assert_eq!(
            parse_iso8601("2020-01-02T03:04:05-03:30")
                .unwrap()
                .offset_minutes,
            Some(-210)
        );
        assert_eq!(
            parse_iso8601("2020-01-02T03:04:05").unwrap().offset_minutes,
            None
        );
        assert_eq!(parse_iso8601("2020-13-02T03:04:05Z"), None);
        assert_eq!(parse_iso8601("hola"), None);
        let u = utc_time(1_557_671_400).unwrap();
        assert_eq!(
            (u.year, u.month, u.day, u.hour, u.minute),
            (2019, 5, 12, 14, 30)
        );
        assert_eq!(
            parse_iso6709("+40.4168-003.7038+650.2/"),
            Some((40.4168, -3.7038))
        );
        assert_eq!(
            parse_iso6709("-33.8688+151.2093/"),
            Some((-33.8688, 151.2093))
        );
        assert_eq!(parse_iso6709("+00.0000+000.0000/"), None);
        assert_eq!(parse_iso6709("basura"), None);
    }
}
