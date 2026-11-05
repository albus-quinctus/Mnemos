//! Título libre de un día (spec 006), como «Ruta del Cares».
//!
//! El título es del día, no de las fotos: la clave es la fecha en hora local
//! tal como agrupa la cuadrícula (`"2024-10-12"`). Solo vive en la base de
//! datos.

use std::time::SystemTime;

use rusqlite::OptionalExtension;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::files::unix_secs;
use crate::library::Library;

/// Longitud máxima de un título, en caracteres.
pub const MAX_DAY_TITLE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DayTitle {
    /// `"AAAA-MM-DD"`.
    pub day: String,
    pub title: String,
}

impl Library {
    /// Todos los títulos, del día más reciente al más antiguo. Son pocos
    /// (los escribe el usuario), así que la interfaz los pide de una vez.
    pub fn day_titles(&self) -> Result<Vec<DayTitle>> {
        let mut stmt = self
            .conn
            .prepare("SELECT day, title FROM day_titles ORDER BY day DESC")?;
        let titles = stmt
            .query_map([], |r| {
                Ok(DayTitle {
                    day: r.get(0)?,
                    title: r.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(titles)
    }

    /// Título de un día, si lo tiene.
    pub fn day_title(&self, day: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT title FROM day_titles WHERE day = ?1", [day], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Guarda el título de `day` y devuelve cómo ha quedado. Los saltos de
    /// línea y tabuladores pasan a ser espacios, se quitan los espacios de los
    /// extremos y se corta a [`MAX_DAY_TITLE`] caracteres. Un título vacío
    /// borra el que hubiera y devuelve `None`.
    pub fn set_day_title(&self, day: &str, title: &str) -> Result<Option<String>> {
        if !self.is_valid_day(day)? {
            return Err(Error::InvalidDay(day.to_owned()));
        }
        let title = normalize(title);
        if title.is_empty() {
            self.conn
                .execute("DELETE FROM day_titles WHERE day = ?1", [day])?;
            return Ok(None);
        }
        self.conn.execute(
            "INSERT INTO day_titles (day, title, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (day) DO UPDATE SET title = excluded.title,
                                             updated_at = excluded.updated_at",
            (day, &title, unix_secs(SystemTime::now())),
        )?;
        Ok(Some(title))
    }

    /// `AAAA-MM-DD` y además una fecha que existe (nada de `2024-02-30`).
    fn is_valid_day(&self, day: &str) -> Result<bool> {
        let shape = day.len() == 10
            && day.bytes().enumerate().all(|(i, b)| {
                if i == 4 || i == 7 {
                    b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            });
        if !shape {
            return Ok(false);
        }
        let normalized: Option<String> = self
            .conn
            .query_row("SELECT date(?1)", [day], |r| r.get(0))?;
        Ok(normalized.as_deref() == Some(day))
    }
}

fn normalize(title: &str) -> String {
    let flat: String = title
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    flat.trim()
        .chars()
        .take(MAX_DAY_TITLE)
        .collect::<String>()
        .trim_end()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        (dir, lib)
    }

    #[test]
    fn guarda_cambia_y_borra() {
        let (_dir, lib) = library();
        assert_eq!(lib.day_title("2024-10-12").unwrap(), None);
        assert_eq!(
            lib.set_day_title("2024-10-12", "Ruta del Cares")
                .unwrap()
                .as_deref(),
            Some("Ruta del Cares")
        );
        lib.set_day_title("2024-10-12", "Ruta del Cares (2.ª parte)")
            .unwrap();
        lib.set_day_title("2023-01-01", "Año nuevo").unwrap();
        assert_eq!(
            lib.day_titles().unwrap(),
            [
                DayTitle {
                    day: "2024-10-12".into(),
                    title: "Ruta del Cares (2.ª parte)".into()
                },
                DayTitle {
                    day: "2023-01-01".into(),
                    title: "Año nuevo".into()
                },
            ]
        );
        assert_eq!(lib.set_day_title("2024-10-12", "   ").unwrap(), None);
        assert_eq!(lib.day_title("2024-10-12").unwrap(), None);
        // Borrar lo que no existe no es un error.
        assert_eq!(lib.set_day_title("2020-05-05", "").unwrap(), None);
    }

    #[test]
    fn limpia_el_texto() {
        let (_dir, lib) = library();
        let guardar = |t: &str| lib.set_day_title("2024-10-12", t).unwrap().unwrap();
        assert_eq!(guardar("  Ruta\ndel\tCares\r\n "), "Ruta del Cares");
        assert_eq!(
            guardar("Cumple 🎂 de «Ana» <b>x</b>"),
            "Cumple 🎂 de «Ana» <b>x</b>"
        );
        assert_eq!(guardar("مرحبا"), "مرحبا");
        let largo = "ñ".repeat(150);
        assert_eq!(guardar(&largo).chars().count(), MAX_DAY_TITLE);
        // Cortar no deja un espacio al final.
        let con_espacio = format!("{} b", "a".repeat(99));
        assert_eq!(guardar(&con_espacio), "a".repeat(99));
    }

    #[test]
    fn rechaza_dias_mal_escritos() {
        let (_dir, lib) = library();
        for day in [
            "12/10/2024",
            "2024-1-12",
            "2024-02-30",
            "2024-13-01",
            "",
            "2024-10-12 ",
            "'; DROP TABLE day_titles; --",
        ] {
            assert!(
                matches!(lib.set_day_title(day, "x"), Err(Error::InvalidDay(_))),
                "{day:?}"
            );
        }
        assert!(lib.set_day_title("2024-02-29", "bisiesto").is_ok());
    }

    fn copiar(desde: &std::path::Path, hasta: &std::path::Path) {
        std::fs::create_dir_all(hasta).unwrap();
        for entrada in std::fs::read_dir(desde).unwrap() {
            let entrada = entrada.unwrap();
            let destino = hasta.join(entrada.file_name());
            if entrada.file_type().unwrap().is_dir() {
                copiar(&entrada.path(), &destino);
            } else {
                std::fs::copy(entrada.path(), destino).unwrap();
            }
        }
    }

    #[test]
    fn favoritos_archivo_y_titulos_viajan_con_la_biblioteca() {
        let equipo_a = tempfile::tempdir().unwrap();
        {
            let mut lib = Library::create(equipo_a.path()).unwrap();
            for i in 1..=3 {
                lib.conn
                    .execute(
                        "INSERT INTO photos (id, rel_path, size_bytes, modified_at, imported_at)
                         VALUES (?1, ?2, 1, 1, 0)",
                        (i, format!("Originales/{i}.jpg")),
                    )
                    .unwrap();
            }
            lib.set_favorite(&[1, 2], true).unwrap();
            lib.set_archived(&[2], true).unwrap();
            lib.set_day_title("2024-10-12", "Ruta del Cares").unwrap();
        }
        // Otro equipo: la misma biblioteca en otra ruta.
        let equipo_b = tempfile::tempdir().unwrap();
        let raiz_b = equipo_b.path().join("FOTOS").join("MNEMOS");
        copiar(equipo_a.path(), &raiz_b);
        drop(equipo_a);

        let lib = Library::open(&raiz_b).unwrap();
        let ids = |sql: &str| -> Vec<i64> {
            lib.conn
                .prepare(sql)
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap()
        };
        assert_eq!(
            ids("SELECT id FROM photos WHERE favorite = 1 ORDER BY id"),
            [1, 2]
        );
        assert_eq!(
            ids("SELECT id FROM photos WHERE archived_at IS NOT NULL"),
            [2]
        );
        assert_eq!(
            lib.day_title("2024-10-12").unwrap().as_deref(),
            Some("Ruta del Cares")
        );
    }
}
