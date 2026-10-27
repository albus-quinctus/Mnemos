//! Fecha de captura guardada en el EXIF (spec 004).

use std::fs::File;
use std::io::{BufRead, BufReader, Seek};
use std::path::Path;

/// Fecha y hora en que se hizo la foto, tal como la guarda la cámara.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    /// Desfase respecto a UTC en minutos, si la cámara lo guarda. Sin él, la
    /// hora es la local de la cámara.
    pub offset_minutes: Option<i16>,
}

impl CaptureTime {
    /// Indica si la fecha existe en el calendario y es posterior a 1900.
    pub fn is_valid(&self) -> bool {
        let days = match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if self.year.is_multiple_of(4)
                && (!self.year.is_multiple_of(100) || self.year.is_multiple_of(400)) =>
            {
                29
            }
            2 => 28,
            _ => return false,
        };
        (1900..=2200).contains(&self.year)
            && (1..=days).contains(&self.day)
            && self.hour < 24
            && self.minute < 60
            && self.second < 60
    }
}

/// Fecha de captura del EXIF: `DateTimeOriginal` o, si falta,
/// `DateTimeDigitized`. Devuelve `None` sin EXIF o con una fecha vacía o
/// imposible.
pub fn capture_time(path: &Path) -> Option<CaptureTime> {
    let file = File::open(path).ok()?;
    capture_time_from_container(&mut BufReader::new(file))
}

pub(crate) fn capture_time_from_container(
    reader: &mut (impl BufRead + Seek),
) -> Option<CaptureTime> {
    let exif = exif::Reader::new().read_from_container(reader).ok()?;
    let ascii = |tag| match &exif.get_field(tag, exif::In::PRIMARY)?.value {
        exif::Value::Ascii(v) => v.first().cloned(),
        _ => None,
    };
    [
        (exif::Tag::DateTimeOriginal, exif::Tag::OffsetTimeOriginal),
        (exif::Tag::DateTimeDigitized, exif::Tag::OffsetTimeDigitized),
    ]
    .into_iter()
    .find_map(|(date, offset)| {
        let mut dt = exif::DateTime::from_ascii(&ascii(date)?).ok()?;
        if let Some(offset) = ascii(offset) {
            let _ = dt.parse_offset(&offset);
        }
        let time = CaptureTime {
            year: dt.year,
            month: dt.month,
            day: dt.day,
            hour: dt.hour,
            minute: dt.minute,
            second: dt.second,
            offset_minutes: dt.offset,
        };
        time.is_valid().then_some(time)
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Cursor;

    /// JPEG mínimo con un EXIF que solo trae las etiquetas ASCII de
    /// `fields` (número de etiqueta y texto) en la sub-IFD Exif.
    pub(crate) fn jpeg_with_exif_dates(fields: &[(u16, &str)]) -> Vec<u8> {
        // TIFF little-endian: cabecera, IFD0 con un puntero a la sub-IFD Exif.
        let mut tiff = b"II*\0\x08\0\0\0".to_vec();
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x8769u16.to_le_bytes());
        tiff.extend_from_slice(&4u16.to_le_bytes()); // LONG
        tiff.extend_from_slice(&1u32.to_le_bytes());
        let exif_ifd = 8 + 2 + 12 + 4;
        tiff.extend_from_slice(&(exif_ifd as u32).to_le_bytes());
        tiff.extend_from_slice(&0u32.to_le_bytes());

        let mut data_at = exif_ifd + 2 + 12 * fields.len() + 4;
        let mut data = Vec::new();
        tiff.extend_from_slice(&(fields.len() as u16).to_le_bytes());
        for (tag, text) in fields {
            let mut bytes = text.as_bytes().to_vec();
            bytes.push(0);
            tiff.extend_from_slice(&tag.to_le_bytes());
            tiff.extend_from_slice(&2u16.to_le_bytes()); // ASCII
            tiff.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
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

        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff);
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE1];
        jpeg.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
        jpeg.extend_from_slice(&app1);
        jpeg.extend_from_slice(&[0xFF, 0xD9]);
        jpeg
    }

    fn read(fields: &[(u16, &str)]) -> Option<CaptureTime> {
        capture_time_from_container(&mut Cursor::new(jpeg_with_exif_dates(fields)))
    }

    #[test]
    fn lee_la_fecha_original() {
        let t = read(&[(0x9003, "2019:05:12 14:30:05")]).unwrap();
        assert_eq!(
            (t.year, t.month, t.day, t.hour, t.minute, t.second),
            (2019, 5, 12, 14, 30, 5)
        );
        assert_eq!(t.offset_minutes, None);
    }

    #[test]
    fn lee_el_desfase_horario() {
        let t = read(&[(0x9003, "2019:05:12 14:30:05"), (0x9011, "+02:00")]).unwrap();
        assert_eq!(t.offset_minutes, Some(120));
    }

    #[test]
    fn usa_la_fecha_de_digitalizacion_si_falta_la_original() {
        let t = read(&[(0x9004, "2018:01:02 03:04:05")]).unwrap();
        assert_eq!((t.year, t.month, t.day), (2018, 1, 2));
    }

    #[test]
    fn ignora_fechas_vacias_o_imposibles() {
        assert_eq!(read(&[(0x9003, "0000:00:00 00:00:00")]), None);
        assert_eq!(read(&[(0x9003, "    :  :     :  :  ")]), None);
        assert_eq!(read(&[(0x9003, "2019:02:30 10:00:00")]), None);
        assert_eq!(read(&[]), None);
    }
}
