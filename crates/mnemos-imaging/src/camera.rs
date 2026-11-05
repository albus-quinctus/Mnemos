//! Datos de cámara del EXIF para el panel de información (spec 011).

use std::fs::File;
use std::io::{BufRead, BufReader, Seek};
use std::path::Path;

use exif::{In, Tag, Value};
use serde::Serialize;

/// Lo que la cámara guardó sobre la foto. Cada campo falta si el EXIF no lo trae.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CameraInfo {
    /// Marca y modelo, sin repetir la marca si el modelo ya la incluye.
    pub camera: Option<String>,
    pub lens: Option<String>,
    /// Tiempo de exposición en segundos.
    pub exposure_s: Option<f64>,
    /// Número f de la apertura.
    pub f_number: Option<f64>,
    pub iso: Option<u32>,
    /// Distancia focal real en milímetros.
    pub focal_mm: Option<f64>,
    /// Equivalente en 35 mm, en milímetros.
    pub focal_35mm: Option<u32>,
    /// Si saltó el flash.
    pub flash: Option<bool>,
    /// Latitud y longitud en grados decimales (sur y oeste en negativo).
    pub gps: Option<(f64, f64)>,
}

impl CameraInfo {
    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Datos de cámara del EXIF de `path`, o `None` si no tiene EXIF legible o no
/// trae ninguno. Solo lee el archivo.
pub fn camera_info(path: &Path) -> Option<CameraInfo> {
    let file = File::open(path).ok()?;
    camera_info_from_container(&mut BufReader::new(file))
}

pub(crate) fn camera_info_from_container(reader: &mut (impl BufRead + Seek)) -> Option<CameraInfo> {
    let exif = exif::Reader::new().read_from_container(reader).ok()?;
    let value = |tag| exif.get_field(tag, In::PRIMARY).map(|f| &f.value);
    let text = |tag| match value(tag)? {
        Value::Ascii(v) => {
            let s = String::from_utf8_lossy(v.first()?);
            let s = s.trim_matches(|c: char| c == '\0' || c.is_whitespace());
            (!s.is_empty()).then(|| s.to_owned())
        }
        _ => None,
    };
    let rational = |tag| match value(tag)? {
        Value::Rational(v) => v
            .first()
            .filter(|r| r.denom != 0)
            .map(|r| r.to_f64())
            .filter(|x| *x > 0.0),
        _ => None,
    };
    let uint = |tag| value(tag)?.get_uint(0);

    let info = CameraInfo {
        camera: camera_name(text(Tag::Make), text(Tag::Model)),
        lens: text(Tag::LensModel),
        exposure_s: rational(Tag::ExposureTime),
        f_number: rational(Tag::FNumber),
        iso: uint(Tag::PhotographicSensitivity).filter(|&v| v > 0),
        focal_mm: rational(Tag::FocalLength),
        focal_35mm: uint(Tag::FocalLengthIn35mmFilm).filter(|&v| v > 0),
        // El bit 0 dice si saltó.
        flash: uint(Tag::Flash).map(|v| v & 1 == 1),
        gps: gps(&exif),
    };
    (!info.is_empty()).then_some(info)
}

/// "Canon" + "Canon EOS 80D" → "Canon EOS 80D"; "SONY" + "ILCE-7M3" → "SONY ILCE-7M3".
fn camera_name(make: Option<String>, model: Option<String>) -> Option<String> {
    match (make, model) {
        (Some(make), Some(model)) => {
            let first = make.split_whitespace().next().unwrap_or(&make);
            if model.to_lowercase().starts_with(&first.to_lowercase()) {
                Some(model)
            } else {
                Some(format!("{make} {model}"))
            }
        }
        (make, model) => model.or(make),
    }
}

fn gps(exif: &exif::Exif) -> Option<(f64, f64)> {
    let coord = |tag, reference, negative: u8| {
        let Value::Rational(v) = &exif.get_field(tag, In::PRIMARY)?.value else {
            return None;
        };
        if v.len() < 3 || v.iter().take(3).any(|r| r.denom == 0) {
            return None;
        }
        let degrees = v[0].to_f64() + v[1].to_f64() / 60.0 + v[2].to_f64() / 3600.0;
        let sign = match &exif.get_field(reference, In::PRIMARY)?.value {
            Value::Ascii(r) if r.first()?.first() == Some(&negative) => -1.0,
            Value::Ascii(_) => 1.0,
            _ => return None,
        };
        Some(sign * degrees)
    };
    let lat = coord(Tag::GPSLatitude, Tag::GPSLatitudeRef, b'S')?;
    let lon = coord(Tag::GPSLongitude, Tag::GPSLongitudeRef, b'W')?;
    // (0, 0) es lo que guardan algunos móviles sin señal.
    ((lat, lon) != (0.0, 0.0) && lat.abs() <= 90.0 && lon.abs() <= 180.0).then_some((lat, lon))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Entrada de un IFD: etiqueta, tipo TIFF y bytes del valor.
    type Entry = (u16, u16, u32, Vec<u8>);

    fn ascii(tag: u16, s: &str) -> Entry {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        (tag, 2, b.len() as u32, b)
    }

    fn short(tag: u16, v: u16) -> Entry {
        (tag, 3, 1, v.to_le_bytes().to_vec())
    }

    fn rationals(tag: u16, vs: &[(u32, u32)]) -> Entry {
        let b = vs
            .iter()
            .flat_map(|(n, d)| [n.to_le_bytes(), d.to_le_bytes()].concat())
            .collect();
        (tag, 5, vs.len() as u32, b)
    }

    /// TIFF little-endian con IFD0, sub-IFD Exif y sub-IFD GPS (las que no
    /// estén vacías), envuelto en un JPEG mínimo.
    fn jpeg(ifd0: Vec<Entry>, exif: Vec<Entry>, gps: Vec<Entry>) -> Vec<u8> {
        let mut ifd0 = ifd0;
        // Los punteros se rellenan después; sus posiciones dependen del tamaño.
        if !exif.is_empty() {
            ifd0.push((0x8769, 4, 1, vec![0; 4]));
        }
        if !gps.is_empty() {
            ifd0.push((0x8825, 4, 1, vec![0; 4]));
        }
        ifd0.sort_by_key(|e| e.0);
        let size = |ifd: &[Entry]| {
            2 + 12 * ifd.len()
                + 4
                + ifd
                    .iter()
                    .map(|e| if e.3.len() > 4 { e.3.len() } else { 0 })
                    .sum::<usize>()
        };
        let at0 = 8;
        let at_exif = at0 + size(&ifd0);
        let at_gps = at_exif + if exif.is_empty() { 0 } else { size(&exif) };
        for e in &mut ifd0 {
            match e.0 {
                0x8769 => e.3 = (at_exif as u32).to_le_bytes().to_vec(),
                0x8825 => e.3 = (at_gps as u32).to_le_bytes().to_vec(),
                _ => {}
            }
        }
        let mut tiff = b"II*\0".to_vec();
        tiff.extend_from_slice(&(at0 as u32).to_le_bytes());
        for ifd in [ifd0, exif, gps] {
            if ifd.is_empty() {
                continue;
            }
            let start = tiff.len();
            let mut data_at = start + 2 + 12 * ifd.len() + 4;
            let mut data = Vec::new();
            tiff.extend_from_slice(&(ifd.len() as u16).to_le_bytes());
            for (tag, ty, count, mut bytes) in ifd {
                tiff.extend_from_slice(&tag.to_le_bytes());
                tiff.extend_from_slice(&ty.to_le_bytes());
                tiff.extend_from_slice(&count.to_le_bytes());
                if bytes.len() <= 4 {
                    bytes.resize(4, 0);
                    tiff.extend_from_slice(&bytes);
                } else {
                    tiff.extend_from_slice(&(data_at as u32).to_le_bytes());
                    data_at += bytes.len();
                    data.extend_from_slice(&bytes);
                }
            }
            tiff.extend_from_slice(&0u32.to_le_bytes());
            tiff.extend_from_slice(&data);
        }
        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff);
        let mut out = vec![0xFF, 0xD8, 0xFF, 0xE1];
        out.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(&app1);
        out.extend_from_slice(&[0xFF, 0xD9]);
        out
    }

    fn read(bytes: Vec<u8>) -> Option<CameraInfo> {
        camera_info_from_container(&mut Cursor::new(bytes))
    }

    #[test]
    fn lee_camara_ajustes_y_gps() {
        let info = read(jpeg(
            vec![ascii(0x010F, "Canon"), ascii(0x0110, "Canon EOS 80D")],
            vec![
                rationals(0x829A, &[(1, 250)]),
                rationals(0x829D, &[(28, 10)]),
                short(0x8827, 400),
                short(0x9209, 0x19),
                rationals(0x920A, &[(50, 1)]),
                short(0xA405, 80),
                ascii(0xA434, "EF-S18-55mm f/3.5-5.6 IS STM"),
            ],
            vec![
                ascii(0x0001, "N"),
                rationals(0x0002, &[(41, 1), (23, 1), (642, 100)]),
                ascii(0x0003, "W"),
                rationals(0x0004, &[(2, 1), (10, 1), (264, 100)]),
            ],
        ))
        .unwrap();
        assert_eq!(info.camera.as_deref(), Some("Canon EOS 80D"));
        assert_eq!(info.lens.as_deref(), Some("EF-S18-55mm f/3.5-5.6 IS STM"));
        assert_eq!(info.exposure_s, Some(0.004));
        assert_eq!(info.f_number, Some(2.8));
        assert_eq!(info.iso, Some(400));
        assert_eq!(info.focal_mm, Some(50.0));
        assert_eq!(info.focal_35mm, Some(80));
        assert_eq!(info.flash, Some(true));
        let (lat, lon) = info.gps.unwrap();
        assert!((lat - 41.385_116).abs() < 1e-5, "{lat}");
        assert!((lon + 2.167_4).abs() < 1e-5, "{lon}");
    }

    #[test]
    fn junta_marca_y_modelo_sin_repetir() {
        let name = |make: Option<&str>, model: Option<&str>| {
            camera_name(make.map(str::to_owned), model.map(str::to_owned))
        };
        assert_eq!(
            name(Some("SONY"), Some("ILCE-7M3")).as_deref(),
            Some("SONY ILCE-7M3")
        );
        assert_eq!(
            name(Some("NIKON CORPORATION"), Some("NIKON D750")).as_deref(),
            Some("NIKON D750")
        );
        assert_eq!(name(Some("Xiaomi"), None).as_deref(), Some("Xiaomi"));
        assert_eq!(name(None, None), None);
    }

    #[test]
    fn ignora_valores_vacios_y_gps_sin_senal() {
        let info = read(jpeg(
            vec![ascii(0x0110, "Pixel 7")],
            vec![rationals(0x829D, &[(0, 0)]), short(0x9209, 0x10)],
            vec![
                ascii(0x0001, "N"),
                rationals(0x0002, &[(0, 1), (0, 1), (0, 1)]),
                ascii(0x0003, "E"),
                rationals(0x0004, &[(0, 1), (0, 1), (0, 1)]),
            ],
        ))
        .unwrap();
        assert_eq!(
            info,
            CameraInfo {
                camera: Some("Pixel 7".to_owned()),
                flash: Some(false),
                ..CameraInfo::default()
            }
        );
    }

    #[test]
    fn sin_exif_no_hay_datos() {
        assert_eq!(read(vec![0xFF, 0xD8, 0xFF, 0xD9]), None);
        // EXIF solo con la fecha: nada que enseñar en la sección Cámara.
        assert_eq!(
            read(jpeg(
                vec![],
                vec![ascii(0x9003, "2019:05:12 14:30:05")],
                vec![]
            )),
            None
        );
        assert_eq!(camera_info(Path::new("no_existe.jpg")), None);
    }
}
