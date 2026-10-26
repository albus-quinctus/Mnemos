//! Búsqueda básica (spec 003): interpreta lo que se escribe en el campo de
//! búsqueda y devuelve sus resultados.
//!
//! El texto se separa en palabras, que se buscan en la ruta con FTS5, y como
//! mucho una fecha, que filtra por el día de la foto. Las palabras nunca se
//! pasan tal cual a FTS5: van entre comillas, así que ningún texto puede
//! producir un error de sintaxis.
//!
//! Los resultados salen como la cuadrícula: meses con su número de fotos y
//! páginas con el mismo cursor, del más reciente al más antiguo.

use rusqlite::types::Value;
use rusqlite::{params, params_from_iter};

use crate::error::Result;
use crate::library::Library;
use crate::timeline::{DATE, Filter, Month, TimelineCursor, TimelinePage};

/// Filtro de fecha reconocido en la consulta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFilter {
    Year(u16),
    Month {
        year: u16,
        month: u8,
    },
    Day {
        year: u16,
        month: u8,
        day: u8,
    },
    /// Mes sin año: se resuelve al más reciente que tenga fotos.
    MonthLatest(u8),
    /// Día sin año: se resuelve al más reciente que tenga fotos.
    DayLatest {
        month: u8,
        day: u8,
    },
}

/// Consulta interpretada.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchQuery {
    /// Palabras normalizadas, en el orden en que se escribieron.
    pub words: Vec<String>,
    pub date: Option<DateFilter>,
}

const MONTHS: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];

const MIN_YEAR: u16 = 1826;
const MAX_YEAR: u16 = 2999;

impl SearchQuery {
    pub fn parse(text: &str) -> Self {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let mut query = SearchQuery::default();
        let mut i = 0;
        while i < tokens.len() {
            let token = tokens[i];
            if query.date.is_none() {
                // Nombre de mes seguido de un año: «marzo 2024», «mar 2024».
                if let Some(month) = month_name(token, true)
                    && let Some(year) = tokens.get(i + 1).and_then(|t| year(t))
                {
                    query.date = Some(DateFilter::Month { year, month });
                    i += 2;
                    continue;
                }
                if let Some(date) = date_token(token) {
                    query.date = Some(date);
                    i += 1;
                    continue;
                }
            }
            // Los caracteres de control no sirven para buscar y un NUL corta
            // la cadena en FTS5.
            let word: String = token.chars().filter(|c| !c.is_control()).collect();
            if word.chars().any(char::is_alphanumeric) {
                query.words.push(word.to_lowercase());
            }
            i += 1;
        }
        query
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty() && self.date.is_none()
    }

    /// Expresión para `photo_search MATCH`, o nada si no hay palabras. Cada
    /// palabra es una frase entre comillas con prefijo, y todas deben estar.
    pub fn fts_match(&self) -> Option<String> {
        if self.words.is_empty() {
            return None;
        }
        let phrases: Vec<String> = self
            .words
            .iter()
            .map(|w| format!("\"{}\"*", w.replace('"', "\"\"")))
            .collect();
        Some(phrases.join(" AND "))
    }
}

impl Library {
    /// Meses con resultados, del más reciente al más antiguo. Con una consulta
    /// vacía son los de toda la biblioteca.
    pub fn search_months(&self, query: &SearchQuery) -> Result<Vec<Month>> {
        match self.search_filter(query)? {
            Some(filter) => self.months_where(&filter),
            None => Ok(Vec::new()),
        }
    }

    /// Hasta `limit` resultados a partir de `after`, como `timeline_page`.
    pub fn search_page(
        &self,
        query: &SearchQuery,
        after: Option<TimelineCursor>,
        limit: u32,
    ) -> Result<TimelinePage> {
        match self.search_filter(query)? {
            Some(filter) => self.page_where(&filter, after, limit),
            None => Ok(TimelinePage {
                photos: Vec::new(),
                next: None,
            }),
        }
    }

    /// Condición SQL de la consulta, o nada si no puede tener resultados: un
    /// mes o un día sin año del que no hay ninguna foto.
    fn search_filter(&self, query: &SearchQuery) -> Result<Option<Filter>> {
        let mut filter = Filter::default();
        if let Some(expr) = query.fts_match() {
            filter
                .sql
                .push_str(" AND id IN (SELECT rowid FROM photo_search WHERE photo_search MATCH ?)");
            filter.params.push(Value::Text(expr));
        }
        let Some(date) = query.date else {
            return Ok(Some(filter));
        };
        let (start, span) = match date {
            DateFilter::Year(year) => (format!("{year:04}-01-01"), "+1 year"),
            DateFilter::Month { year, month } => (format!("{year:04}-{month:02}-01"), "+1 month"),
            DateFilter::Day { year, month, day } => {
                (format!("{year:04}-{month:02}-{day:02}"), "+1 day")
            }
            DateFilter::MonthLatest(month) => {
                let Some(year) = self.latest_year(&filter, "%m", &format!("{month:02}"))? else {
                    return Ok(None);
                };
                (format!("{year}-{month:02}-01"), "+1 month")
            }
            DateFilter::DayLatest { month, day } => {
                let Some(year) =
                    self.latest_year(&filter, "%m-%d", &format!("{month:02}-{day:02}"))?
                else {
                    return Ok(None);
                };
                (format!("{year}-{month:02}-{day:02}"), "+1 day")
            }
        };
        // El periodo va en hora local; 'utc' lo pasa a segundos UTC, que es
        // como se guardan las fechas. Así la consulta usa el índice por fecha.
        let (from, to): (i64, i64) = self.conn.query_row(
            "SELECT unixepoch(?1, 'utc'), unixepoch(?1, ?2, 'utc')",
            params![start, span],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        filter
            .sql
            .push_str(&format!(" AND {DATE} >= ? AND {DATE} < ?"));
        filter.params.extend([Value::from(from), Value::from(to)]);
        Ok(Some(filter))
    }

    /// Año más reciente con resultados cuya fecha local, con el formato de
    /// `strftime` dado, vale `value`.
    fn latest_year(&self, filter: &Filter, format: &str, value: &str) -> Result<Option<String>> {
        let params = [
            Value::from(format.to_owned()),
            Value::from(value.to_owned()),
        ]
        .into_iter()
        .chain(filter.params.iter().cloned());
        let year = self.conn.query_row(
            &format!(
                "SELECT max(strftime('%Y', {DATE}, 'unixepoch', 'localtime')) FROM photos
                 WHERE strftime(?, {DATE}, 'unixepoch', 'localtime') = ? {}",
                filter.sql
            ),
            params_from_iter(params),
            |r| r.get(0),
        )?;
        Ok(year)
    }
}

/// Fecha escrita en un solo token, o nada si no lo es.
fn date_token(token: &str) -> Option<DateFilter> {
    if let Some(year) = year(token) {
        return Some(DateFilter::Year(year));
    }
    // Un nombre de mes completo vale solo: «marzo». Las abreviaturas, no,
    // porque «mar» también es una palabra corriente.
    if let Some(month) = month_name(token, false) {
        return Some(DateFilter::MonthLatest(month));
    }
    let parts: Vec<&str> = token.split(['-', '/']).collect();
    let dashes = token.contains('-');
    let slashes = token.contains('/');
    if dashes == slashes {
        return None;
    }
    let nums: Option<Vec<u32>> = parts
        .iter()
        .map(|p| {
            (!p.is_empty() && p.len() <= 4 && p.bytes().all(|b| b.is_ascii_digit()))
                .then(|| p.parse().ok())
                .flatten()
        })
        .collect();
    let nums = nums?;
    match (dashes, nums.as_slice(), parts.as_slice()) {
        // 2024-03 y 2024-03-26: año delante, con cuatro cifras.
        (true, [y, m], [ys, _]) if ys.len() == 4 => month_of(*y, *m),
        (true, [y, m, d], [ys, _, _]) if ys.len() == 4 => day_of(*y, *m, *d),
        // 03/2024, 26/10/2026 y 26/10: día y mes delante.
        (false, [m, y], [_, ys]) if ys.len() == 4 => month_of(*y, *m),
        (false, [d, m, y], [_, _, ys]) if ys.len() == 4 => day_of(*y, *m, *d),
        (false, [d, m], [_, ms]) if ms.len() <= 2 => {
            // Sin año se admite el 29 de febrero.
            valid_day(2024, *m, *d).then_some(DateFilter::DayLatest {
                month: *m as u8,
                day: *d as u8,
            })
        }
        _ => None,
    }
}

fn year(token: &str) -> Option<u16> {
    if token.len() != 4 || !token.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let year: u16 = token.parse().ok()?;
    (MIN_YEAR..=MAX_YEAR).contains(&year).then_some(year)
}

fn month_of(year: u32, month: u32) -> Option<DateFilter> {
    let year = u16::try_from(year).ok()?;
    ((MIN_YEAR..=MAX_YEAR).contains(&year) && (1..=12).contains(&month)).then_some(
        DateFilter::Month {
            year,
            month: month as u8,
        },
    )
}

fn day_of(year: u32, month: u32, day: u32) -> Option<DateFilter> {
    let DateFilter::Month { year, month } = month_of(year, month)? else {
        return None;
    };
    valid_day(year as u32, month as u32, day).then_some(DateFilter::Day {
        year,
        month,
        day: day as u8,
    })
}

fn valid_day(year: u32, month: u32, day: u32) -> bool {
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

/// Número de mes de un nombre en español, sin distinguir mayúsculas. Con
/// `abbreviated`, admite también las tres primeras letras y «sept».
fn month_name(token: &str, abbreviated: bool) -> Option<u8> {
    let token = token.trim_end_matches('.').to_lowercase();
    MONTHS
        .iter()
        .position(|name| {
            *name == token
                || (abbreviated
                    && ((token.len() == 3 && name.starts_with(&token))
                        || (token == "sept" && *name == "septiembre")))
        })
        .map(|i| i as u8 + 1)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;

    use super::*;

    fn fecha(text: &str) -> Option<DateFilter> {
        SearchQuery::parse(text).date
    }

    #[test]
    fn separa_palabras_en_minusculas() {
        let q = SearchQuery::parse("  Playa   CUMPLEAÑOS ");
        assert_eq!(q.words, ["playa", "cumpleaños"]);
        assert_eq!(q.date, None);
        assert!(SearchQuery::parse("   ").is_empty());
    }

    #[test]
    fn reconoce_las_fechas() {
        use DateFilter::*;
        assert_eq!(fecha("2024"), Some(Year(2024)));
        let marzo = Some(Month {
            year: 2024,
            month: 3,
        });
        for texto in [
            "marzo 2024",
            "Marzo 2024",
            "mar 2024",
            "mar. 2024",
            "2024-03",
            "03/2024",
            "3/2024",
        ] {
            assert_eq!(fecha(texto), marzo, "{texto}");
        }
        assert_eq!(
            fecha("sept 2025"),
            Some(Month {
                year: 2025,
                month: 9
            })
        );
        let dia = Some(Day {
            year: 2026,
            month: 10,
            day: 26,
        });
        assert_eq!(fecha("26/10/2026"), dia);
        assert_eq!(fecha("2026-10-26"), dia);
        assert_eq!(fecha("marzo"), Some(MonthLatest(3)));
        assert_eq!(fecha("26/10"), Some(DayLatest { month: 10, day: 26 }));
        assert_eq!(fecha("29/02"), Some(DayLatest { month: 2, day: 29 }));
    }

    #[test]
    fn combina_palabras_y_fecha() {
        let q = SearchQuery::parse("playa 2024 Málaga");
        assert_eq!(q.words, ["playa", "málaga"]);
        assert_eq!(q.date, Some(DateFilter::Year(2024)));
        // Solo una fecha: la segunda se busca como palabra.
        let q = SearchQuery::parse("2024 2025");
        assert_eq!(q.date, Some(DateFilter::Year(2024)));
        assert_eq!(q.words, ["2025"]);
    }

    #[test]
    fn lo_que_no_es_fecha_es_palabra() {
        for texto in [
            "31/02/2024",
            "2024-13",
            "0001",
            "12345",
            "IMG_2024",
            "mar",
            "2024/03",
            "26-10-2026",
            "1/2/3/4",
        ] {
            let q = SearchQuery::parse(texto);
            assert_eq!(q.date, None, "{texto}");
            assert_eq!(q.words.len(), 1, "{texto}");
        }
    }

    #[test]
    fn descarta_lo_que_no_tiene_letras_ni_numeros() {
        let q = SearchQuery::parse("* - () \"\" playa");
        assert_eq!(q.words, ["playa"]);
    }

    #[test]
    fn escapa_las_palabras_para_fts() {
        let q = SearchQuery::parse("img_00 di\"jo AND");
        assert_eq!(
            q.fts_match().unwrap(),
            r#""img_00"* AND "di""jo"* AND "and"*"#
        );
        assert_eq!(SearchQuery::parse("2024").fts_match(), None);
    }

    fn fts() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE VIRTUAL TABLE photo_search USING fts5 (
                 rel_path, tags, tokenize = 'unicode61 remove_diacritics 2');
             INSERT INTO photo_search (rowid, rel_path, tags) VALUES
                 (1, 'Originales/2024/03/Cumpleaños de Ana/IMG_0012.jpg', ''),
                 (2, 'Originales/2025/07/Playa/DSC_0001.NEF', '');",
        )
        .unwrap();
        conn
    }

    fn buscar(conn: &Connection, text: &str) -> Vec<i64> {
        let Some(expr) = SearchQuery::parse(text).fts_match() else {
            return Vec::new();
        };
        let mut stmt = conn
            .prepare("SELECT rowid FROM photo_search WHERE photo_search MATCH ?1 ORDER BY rowid")
            .unwrap();
        stmt.query_map([expr], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap_or_else(|e| panic!("{text:?}: {e}"))
    }

    #[test]
    fn busca_por_inicio_de_palabra_sin_acentos() {
        let conn = fts();
        assert_eq!(buscar(&conn, "cumple"), [1]);
        assert_eq!(buscar(&conn, "CUMPLEANOS"), [1]);
        assert_eq!(buscar(&conn, "img_00"), [1]);
        assert_eq!(buscar(&conn, "ana cumple"), [1]);
        assert_eq!(buscar(&conn, "playa dsc"), [2]);
        assert_eq!(buscar(&conn, "playa ana"), Vec::<i64>::new());
        assert_eq!(buscar(&conn, "originales"), [1, 2]);
    }

    #[test]
    fn ningun_texto_rompe_fts() {
        let conn = fts();
        let raros = [
            "\"",
            "\"\"",
            "*",
            "a*b",
            "AND",
            "OR NOT",
            "NEAR(a b)",
            "a:b",
            "rel_path:x",
            "^a",
            "(",
            ")",
            "{a}",
            "a - b",
            "'",
            "\\",
            "%",
            "_",
            "a\"b\"c",
            "ñ",
            "日本語",
            "🙂",
            "a\u{0}b",
        ];
        for texto in raros {
            buscar(&conn, texto);
        }
        // Textos al azar con caracteres de todo tipo.
        let alfabeto: Vec<char> = "aZ9 _-*\"'():^{}+.,/\\ñé日🙂\t\n".chars().collect();
        let mut semilla: u64 = 0x5eed;
        for _ in 0..2000 {
            let largo = (semilla % 12) as usize;
            let texto: String = (0..largo)
                .map(|_| {
                    semilla = semilla
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    alfabeto[(semilla >> 33) as usize % alfabeto.len()]
                })
                .collect();
            buscar(&conn, &texto);
        }
    }

    /// Biblioteca indexada de verdad con fotos en `Originales/` y la fecha de
    /// modificación dada en hora local («2024-03-15 12:00»).
    fn biblioteca(fotos: &[(&str, &str)]) -> (tempfile::TempDir, Library) {
        use crate::index::tests::{set_mtime, write_png};
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        for (i, (ruta, fecha)) in fotos.iter().enumerate() {
            let ruta = dir.path().join("Originales").join(ruta);
            write_png(&ruta, 4, 3, &i.to_le_bytes());
            let secs: u64 = lib
                .connection()
                .query_row("SELECT unixepoch(?1, 'utc')", [fecha], |r| r.get(0))
                .unwrap();
            set_mtime(&ruta, secs);
        }
        assert_eq!(lib.index().unwrap().added as usize, fotos.len());
        (dir, lib)
    }

    /// Nombres de los resultados, recorriendo todas las páginas de 2 en 2.
    fn resultados(lib: &Library, texto: &str) -> Vec<String> {
        let query = SearchQuery::parse(texto);
        let mut nombres = Vec::new();
        let mut after = None;
        loop {
            let page = lib.search_page(&query, after, 2).unwrap();
            nombres.extend(page.photos.iter().map(|p| p.name.clone()));
            match page.next {
                Some(next) => after = Some(next),
                None => break,
            }
        }
        let total: u64 = lib
            .search_months(&query)
            .unwrap()
            .iter()
            .map(|m| m.photos)
            .sum();
        assert_eq!(total as usize, nombres.len(), "{texto:?}");
        nombres
    }

    fn ejemplo() -> (tempfile::TempDir, Library) {
        biblioteca(&[
            ("Cumpleaños/IMG_0001.png", "2026-10-20 12:00"),
            ("Playa/a.png", "2024-07-10 12:00"),
            ("Playa/b.png", "2025-07-10 12:00"),
            ("Varios/marzo-viejo.png", "2024-03-01 00:30"),
            ("Varios/marzo-nuevo.png", "2026-03-31 23:30"),
            ("Varios/abril.png", "2026-04-01 00:30"),
            ("Varios/dia.png", "2025-10-26 08:00"),
        ])
    }

    #[test]
    fn busca_en_la_biblioteca_por_palabras() {
        let (_dir, lib) = ejemplo();
        assert_eq!(resultados(&lib, "cumple"), ["IMG_0001.png"]);
        assert_eq!(resultados(&lib, "CUMPLEANOS"), ["IMG_0001.png"]);
        assert_eq!(resultados(&lib, "playa"), ["b.png", "a.png"]);
        assert!(resultados(&lib, "nada").is_empty());
        assert_eq!(resultados(&lib, "  ").len(), 7);
    }

    #[test]
    fn combina_palabras_con_la_fecha() {
        let (_dir, lib) = ejemplo();
        assert_eq!(resultados(&lib, "playa 2024"), ["a.png"]);
        assert_eq!(resultados(&lib, "2024 playa"), ["a.png"]);
        let meses = lib.search_months(&SearchQuery::parse("playa")).unwrap();
        let meses: Vec<_> = meses.iter().map(|m| (m.month.as_str(), m.photos)).collect();
        assert_eq!(meses, [("2025-07", 1), ("2024-07", 1)]);
    }

    #[test]
    fn las_formas_de_un_mes_dan_lo_mismo_y_respetan_sus_limites() {
        let (_dir, lib) = ejemplo();
        for texto in ["marzo 2026", "mar 2026", "2026-03", "03/2026"] {
            assert_eq!(resultados(&lib, texto), ["marzo-nuevo.png"], "{texto}");
        }
        assert_eq!(resultados(&lib, "2024-03"), ["marzo-viejo.png"]);
        assert_eq!(resultados(&lib, "26/10/2025"), ["dia.png"]);
        assert_eq!(resultados(&lib, "2025-10-26"), ["dia.png"]);
    }

    #[test]
    fn un_mes_o_un_dia_sin_ano_es_el_mas_reciente_con_fotos() {
        let (_dir, lib) = ejemplo();
        assert_eq!(resultados(&lib, "marzo"), ["marzo-nuevo.png"]);
        // Con palabras, el más reciente que tenga resultados.
        assert_eq!(resultados(&lib, "viejo marzo"), ["marzo-viejo.png"]);
        assert_eq!(resultados(&lib, "julio"), ["b.png"]);
        assert_eq!(resultados(&lib, "26/10"), ["dia.png"]);
        assert_eq!(resultados(&lib, "10/07"), ["b.png"]);
        assert!(resultados(&lib, "enero").is_empty());
        assert!(resultados(&lib, "cumple julio").is_empty());
    }

    #[test]
    fn buscar_no_toca_los_originales() {
        let (dir, lib) = ejemplo();
        let ruta = dir.path().join("Originales/Playa/a.png");
        let antes = (
            fs::read(&ruta).unwrap(),
            fs::metadata(&ruta).unwrap().modified().unwrap(),
        );
        for texto in ["playa", "2024", "marzo", "\"\"", "nada"] {
            resultados(&lib, texto);
        }
        let despues = (
            fs::read(&ruta).unwrap(),
            fs::metadata(&ruta).unwrap().modified().unwrap(),
        );
        assert_eq!(antes, despues);
    }

    #[test]
    fn los_textos_raros_no_dan_error_en_la_biblioteca() {
        let (_dir, lib) = ejemplo();
        for texto in [
            "\"",
            "*",
            "NEAR(a b)",
            "rel_path:x",
            "a\u{0}b",
            "31/02/2024",
            "2024-13",
            "^",
        ] {
            resultados(&lib, texto);
        }
    }
}
