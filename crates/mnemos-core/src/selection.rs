//! Ids para la selección múltiple (spec 006).
//!
//! La cuadrícula solo tiene cargadas algunas páginas, así que «todo el día»,
//! `Ctrl` + `A` y el rango de `Mayús` + clic se resuelven aquí con una
//! consulta, sin cargar nada más. Los ids salen en el orden de la cuadrícula,
//! de lo más reciente a lo más antiguo.

use rusqlite::params_from_iter;
use rusqlite::types::Value;

use crate::error::Result;
use crate::library::Library;
use crate::search::SearchQuery;
use crate::timeline::{Filter, TimelineCursor, View};

/// Dónde se está seleccionando: una vista o los resultados de una búsqueda.
#[derive(Debug, Clone, Copy)]
pub enum Scope<'a> {
    View(View),
    Search(&'a SearchQuery),
}

impl Library {
    /// Todas las fotos de `scope` (`Ctrl` + `A`).
    pub fn select_all(&self, scope: Scope<'_>) -> Result<Vec<i64>> {
        self.select_where(scope, |_| String::new(), Vec::new())
    }

    /// Las fotos de `scope` de un día en hora local, `"2024-10-12"`, como
    /// agrupa la cuadrícula. Un día mal escrito no da ninguna.
    pub fn select_day(&self, scope: Scope<'_>, day: &str) -> Result<Vec<i64>> {
        self.select_where(
            scope,
            |order| format!(" AND strftime('%Y-%m-%d', {order}, 'unixepoch', 'localtime') = ?"),
            vec![Value::from(day.to_owned())],
        )
    }

    /// Las fotos de `scope` entre `a` y `b`, ambas incluidas, sea cual sea la
    /// que va primero (`Mayús` + clic). Cada extremo es la fecha y el id de una
    /// foto, como los da la cuadrícula.
    pub fn select_range(
        &self,
        scope: Scope<'_>,
        a: TimelineCursor,
        b: TimelineCursor,
    ) -> Result<Vec<i64>> {
        let (newer, older) = if (a.date, a.id) >= (b.date, b.id) {
            (a, b)
        } else {
            (b, a)
        };
        self.select_where(
            scope,
            |order| format!(" AND ({order}, id) <= (?, ?) AND ({order}, id) >= (?, ?)"),
            vec![
                Value::from(newer.date),
                Value::from(newer.id),
                Value::from(older.date),
                Value::from(older.id),
            ],
        )
    }

    /// `sql` recibe la fecha por la que ordena la vista (la de la Papelera no
    /// es la de captura) y devuelve la condición extra.
    fn select_where(
        &self,
        scope: Scope<'_>,
        sql: impl FnOnce(&str) -> String,
        params: Vec<Value>,
    ) -> Result<Vec<i64>> {
        let filter = match scope {
            Scope::View(view) => view.filter(),
            Scope::Search(query) => match self.search_filter(query)? {
                Some(filter) => filter,
                None => return Ok(Vec::new()),
            },
        };
        let Filter {
            sql: scope_sql,
            params: scope_params,
            order,
        } = filter;
        let sql = sql(order);
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id FROM photos WHERE 1 {scope_sql} {sql} ORDER BY {order} DESC, id DESC"
        ))?;
        let ids = stmt
            .query_map(
                params_from_iter(scope_params.into_iter().chain(params)),
                |r| r.get(0),
            )?
            .collect::<rusqlite::Result<_>>()?;
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::DATE;
    use rusqlite::params;

    /// 2024-10-12 12:00 UTC: a mediodía, para que la zona horaria del equipo
    /// no cambie el día.
    const DIA: i64 = 1_728_734_400;
    const HORA: i64 = 3600;

    /// Fotos 1 a 6: tres el 12-10-2024 (1 la más reciente), dos el 11-10 y
    /// una el 10-10. La 2 y la 3 comparten fecha para probar el desempate.
    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        let fechas = [
            DIA + 2 * HORA,
            DIA,
            DIA,
            DIA - 86_400,
            DIA - 86_400 - HORA,
            DIA - 2 * 86_400,
        ];
        for (i, fecha) in fechas.iter().enumerate() {
            lib.conn
                .execute(
                    "INSERT INTO photos (id, rel_path, size_bytes, modified_at, imported_at)
                     VALUES (?1, ?2, 1, ?3, 0)",
                    params![i as i64 + 1, format!("Originales/playa/{i}.jpg"), fecha],
                )
                .unwrap();
        }
        // Índice de búsqueda como lo deja el indexado.
        lib.conn
            .execute_batch(
                "INSERT INTO photo_search (rowid, rel_path, tags)
                 SELECT id, rel_path, '' FROM photos",
            )
            .unwrap();
        (dir, lib)
    }

    fn cursor(lib: &Library, id: i64) -> TimelineCursor {
        let date = lib
            .conn
            .query_row(
                &format!("SELECT {DATE} FROM photos WHERE id = ?1"),
                [id],
                |r| r.get(0),
            )
            .unwrap();
        TimelineCursor { date, id }
    }

    const FOTOS: Scope<'static> = Scope::View(View::Photos);

    #[test]
    fn todo_en_el_orden_de_la_cuadricula() {
        let (_dir, lib) = library();
        // La 3 va antes que la 2 porque, con la misma fecha, desempata el id.
        assert_eq!(lib.select_all(FOTOS).unwrap(), [1, 3, 2, 4, 5, 6]);
    }

    #[test]
    fn todo_respeta_la_vista() {
        let (_dir, mut lib) = library();
        lib.set_favorite(&[2, 5], true).unwrap();
        lib.set_archived(&[5, 6], true).unwrap();
        assert_eq!(lib.select_all(FOTOS).unwrap(), [1, 3, 2, 4]);
        assert_eq!(
            lib.select_all(Scope::View(View::Favorites)).unwrap(),
            [2, 5]
        );
        assert_eq!(lib.select_all(Scope::View(View::Archive)).unwrap(), [5, 6]);
    }

    #[test]
    fn un_dia() {
        let (_dir, mut lib) = library();
        assert_eq!(lib.select_day(FOTOS, "2024-10-12").unwrap(), [1, 3, 2]);
        assert_eq!(lib.select_day(FOTOS, "2024-10-11").unwrap(), [4, 5]);
        assert!(lib.select_day(FOTOS, "2024-10-09").unwrap().is_empty());
        assert!(lib.select_day(FOTOS, "12/10/2024").unwrap().is_empty());
        assert!(
            lib.select_day(FOTOS, "'; DROP TABLE photos; --")
                .unwrap()
                .is_empty()
        );

        lib.set_archived(&[4], true).unwrap();
        assert_eq!(lib.select_day(FOTOS, "2024-10-11").unwrap(), [5]);
    }

    #[test]
    fn un_rango_en_cualquier_sentido() {
        let (_dir, lib) = library();
        let (a, b) = (cursor(&lib, 3), cursor(&lib, 5));
        assert_eq!(lib.select_range(FOTOS, a, b).unwrap(), [3, 2, 4, 5]);
        assert_eq!(lib.select_range(FOTOS, b, a).unwrap(), [3, 2, 4, 5]);
        // Las dos con la misma fecha.
        let (c, d) = (cursor(&lib, 2), cursor(&lib, 3));
        assert_eq!(lib.select_range(FOTOS, c, d).unwrap(), [3, 2]);
        assert_eq!(lib.select_range(FOTOS, a, a).unwrap(), [3]);
    }

    #[test]
    fn el_rango_salta_lo_que_no_esta_en_la_vista() {
        let (_dir, mut lib) = library();
        lib.set_archived(&[2, 4], true).unwrap();
        let rango = lib
            .select_range(FOTOS, cursor(&lib, 1), cursor(&lib, 6))
            .unwrap();
        assert_eq!(rango, [1, 3, 5, 6]);
    }

    #[test]
    fn dentro_de_una_busqueda() {
        let (_dir, mut lib) = library();
        lib.conn
            .execute(
                "UPDATE photo_search SET rel_path = 'Originales/monte/x.jpg' WHERE rowid = 3",
                [],
            )
            .unwrap();
        lib.set_archived(&[2], true).unwrap();
        let playa = SearchQuery::parse("playa");
        let busqueda = Scope::Search(&playa);
        // La búsqueda incluye la 2 aunque esté archivada y no la 3.
        assert_eq!(lib.select_all(busqueda).unwrap(), [1, 2, 4, 5, 6]);
        assert_eq!(lib.select_day(busqueda, "2024-10-12").unwrap(), [1, 2]);
        assert_eq!(
            lib.select_range(busqueda, cursor(&lib, 2), cursor(&lib, 4))
                .unwrap(),
            [2, 4]
        );
        // Una búsqueda sin resultados posibles.
        let nada = SearchQuery::parse("diciembre");
        assert!(lib.select_all(Scope::Search(&nada)).unwrap().is_empty());
    }
}
