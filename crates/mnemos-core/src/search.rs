//! Búsqueda básica (spec 003): interpreta lo que se escribe en el campo de
//! búsqueda.
//!
//! El texto se separa en palabras, que se buscan en la ruta con FTS5, y como
//! mucho una fecha, que filtra por el día de la foto. Las palabras nunca se
//! pasan tal cual a FTS5: van entre comillas, así que ningún texto puede
//! producir un error de sintaxis.

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
}
