//! Utilidades de archivos compartidas por el indexado y la importación.

use std::fs::File;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use walkdir::DirEntry;

/// Huella BLAKE3 del contenido del archivo, en hexadecimal.
pub(crate) fn hash_file(path: &Path) -> io::Result<String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(File::open(path)?)?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// Segundos desde la época Unix; negativos para fechas anteriores a 1970.
pub(crate) fn unix_secs(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

/// Año y mes (UTC) de una fecha en segundos Unix.
///
/// Algoritmo `civil_from_days` de Howard Hinnant, para no depender de una
/// biblioteca de fechas solo por esto.
pub(crate) fn year_month(secs: i64) -> (i64, u32) {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month as u32)
}

/// Días desde el 1970-01-01 de una fecha del calendario gregoriano; la
/// inversa de [`year_month`] (algoritmo `days_from_civil`, también de Hinnant).
pub(crate) fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let mp = i64::from((month + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Archivos y carpetas ocultos (empiezan por `.`), salvo la raíz del recorrido.
pub(crate) fn is_hidden(entry: &DirEntry) -> bool {
    entry.depth() > 0 && entry.file_name().to_string_lossy().starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calcula_anio_y_mes() {
        assert_eq!(year_month(0), (1970, 1));
        assert_eq!(year_month(-1), (1969, 12));
        assert_eq!(year_month(951_782_400), (2000, 2)); // 2000-02-29
        assert_eq!(year_month(1_700_000_000), (2023, 11));
        assert_eq!(year_month(1_704_067_199), (2023, 12)); // 2023-12-31 23:59:59
        assert_eq!(year_month(1_704_067_200), (2024, 1));
    }

    #[test]
    fn calcula_los_dias_desde_1970() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1969, 12, 31), -1);
        assert_eq!(days_from_civil(2000, 2, 29) * 86_400, 951_782_400);
        assert_eq!(days_from_civil(2024, 1, 1) * 86_400, 1_704_067_200);
    }
}
