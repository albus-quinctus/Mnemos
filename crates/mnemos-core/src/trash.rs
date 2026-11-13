//! Papelera (spec 015).
//!
//! Enviar una foto a la Papelera solo pone la fecha en `photos.trashed_at`:
//! el archivo sigue en su sitio. Las vistas, la búsqueda y los duplicados
//! dejan fuera esas fotos. Borrar el archivo es la única excepción a la regla
//! 1 de `CLAUDE.md` y vivirá solo en este módulo.
//!
//! Enviar y restaurar funcionan como archivar (spec 006): una transacción
//! para toda la selección y, de vuelta, los ids que cambiaron de verdad para
//! poder deshacer.

use std::time::SystemTime;

use crate::error::Result;
use crate::files::unix_secs;
use crate::library::Library;

/// Días que pasa una foto en la Papelera antes de eliminarse para siempre.
pub const TRASH_DAYS: i64 = 30;

impl Library {
    /// Envía a la Papelera las fotos `ids`. No toca los archivos: solo guarda
    /// la fecha de envío. Una foto que ya estaba conserva su fecha. Los ids
    /// que no existen se ignoran.
    pub fn send_to_trash(&mut self, ids: &[i64]) -> Result<Vec<i64>> {
        self.update_each(
            ids,
            "UPDATE photos SET trashed_at = ?2 WHERE id = ?1 AND trashed_at IS NULL",
            Some(unix_secs(SystemTime::now())),
        )
    }

    /// Saca de la Papelera las fotos `ids`. Vuelven con su favorito, su
    /// archivo y su edición, que no se tocaron al enviarlas.
    pub fn restore_from_trash(&mut self, ids: &[i64]) -> Result<Vec<i64>> {
        self.update_each(
            ids,
            "UPDATE photos SET trashed_at = NULL WHERE id = ?1 AND trashed_at IS NOT NULL",
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use crate::library::Library;
    use crate::search::SearchQuery;
    use crate::selection::Scope;
    use crate::timeline::View;

    /// Seis fotos: la 1 y la 2 duplicadas, la 3 favorita, la 4 archivada, la
    /// 5 favorita y archivada. Todas se llaman «playa». Después se envían a
    /// la Papelera la 2, la 3, la 4 y la 5.
    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Originales")).unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        let tx = lib.conn.transaction().unwrap();
        for i in 1..=6_i64 {
            let hash = if i <= 2 {
                "h".to_owned()
            } else {
                format!("h{i}")
            };
            tx.execute(
                "INSERT INTO photos (id, rel_path, size_bytes, modified_at, imported_at,
                                     content_hash, favorite, archived_at)
                 VALUES (?1, ?2, 1, ?1 * 86400, 0, ?3, ?4, ?5)",
                params![
                    i,
                    format!("Originales/playa {i}.jpg"),
                    hash,
                    i == 3 || i == 5,
                    (i == 4 || i == 5).then_some(10),
                ],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO photo_search (rowid, rel_path, tags) VALUES (?1, ?2, '')",
                params![i, format!("Originales/playa {i}.jpg")],
            )
            .unwrap();
        }
        tx.execute(
            "UPDATE photos SET trashed_at = 100 WHERE id IN (2, 3, 4, 5)",
            [],
        )
        .unwrap();
        tx.commit().unwrap();
        (dir, lib)
    }

    fn page_ids(lib: &Library, view: View) -> Vec<i64> {
        let page = lib.view_page(view, None, 100).unwrap();
        page.photos.iter().map(|p| p.id).collect()
    }

    fn months_total(lib: &Library, view: View) -> u64 {
        lib.view_months(view)
            .unwrap()
            .iter()
            .map(|m| m.photos)
            .sum()
    }

    #[test]
    fn las_vistas_y_la_barra_de_meses_dejan_fuera_la_papelera() {
        let (_dir, lib) = library();
        assert_eq!(page_ids(&lib, View::Photos), [6, 1]);
        assert_eq!(page_ids(&lib, View::Favorites), Vec::<i64>::new());
        assert_eq!(page_ids(&lib, View::Archive), Vec::<i64>::new());
        assert_eq!(months_total(&lib, View::Photos), 2);
        assert_eq!(months_total(&lib, View::Favorites), 0);
        assert_eq!(months_total(&lib, View::Archive), 0);
        assert_eq!(lib.select_all(Scope::View(View::Photos)).unwrap(), [6, 1]);
    }

    #[test]
    fn la_busqueda_deja_fuera_la_papelera() {
        let (_dir, lib) = library();
        for text in ["playa", ""] {
            let query = SearchQuery::parse(text);
            let page = lib.search_page(&query, None, 100).unwrap();
            let ids: Vec<i64> = page.photos.iter().map(|p| p.id).collect();
            assert_eq!(ids, [6, 1], "búsqueda {text:?}");
            let total: u64 = lib
                .search_months(&query)
                .unwrap()
                .iter()
                .map(|m| m.photos)
                .sum();
            assert_eq!(total, 2, "búsqueda {text:?}");
            assert_eq!(lib.select_all(Scope::Search(&query)).unwrap(), [6, 1]);
        }
    }

    #[test]
    fn una_copia_en_la_papelera_no_cuenta_como_duplicado() {
        let (_dir, lib) = library();
        let summary = lib.duplicate_summary().unwrap();
        assert_eq!((summary.groups, summary.extra_copies), (0, 0));
        assert!(lib.duplicate_groups(None, 10).unwrap().groups.is_empty());
    }

    #[test]
    fn los_duplicados_vuelven_al_sacar_la_copia_de_la_papelera() {
        let (_dir, lib) = library();
        lib.conn
            .execute("UPDATE photos SET trashed_at = NULL WHERE id = 2", [])
            .unwrap();
        assert_eq!(lib.duplicate_summary().unwrap().groups, 1);
        let groups = lib.duplicate_groups(None, 10).unwrap().groups;
        assert_eq!(groups[0].copies.len(), 2);
    }

    #[test]
    fn enviar_y_restaurar_devuelven_solo_lo_que_cambia() {
        let (_dir, mut lib) = library();
        // La 2 ya estaba en la Papelera y la 99 no existe.
        assert_eq!(lib.send_to_trash(&[1, 2, 99, 1]).unwrap(), [1]);
        assert_eq!(page_ids(&lib, View::Photos), [6]);
        // Deshacer: solo vuelve la 1; la 2 sigue en la Papelera.
        assert_eq!(lib.restore_from_trash(&[1]).unwrap(), [1]);
        assert_eq!(page_ids(&lib, View::Photos), [6, 1]);
        assert_eq!(lib.restore_from_trash(&[1, 6]).unwrap(), Vec::<i64>::new());
    }

    #[test]
    fn enviar_conserva_la_fecha_de_envio_de_las_que_ya_estaban() {
        let (_dir, mut lib) = library();
        lib.send_to_trash(&[2, 6]).unwrap();
        let fecha = |id: i64| -> i64 {
            lib.conn
                .query_row("SELECT trashed_at FROM photos WHERE id = ?1", [id], |r| {
                    r.get(0)
                })
                .unwrap()
        };
        assert_eq!(fecha(2), 100);
        assert!(fecha(6) > 1_700_000_000);
    }

    #[test]
    fn restaurar_devuelve_favorito_y_archivo() {
        let (_dir, mut lib) = library();
        lib.restore_from_trash(&[3, 4, 5]).unwrap();
        assert_eq!(page_ids(&lib, View::Favorites), [5, 3]);
        assert_eq!(page_ids(&lib, View::Archive), [5, 4]);
        assert_eq!(page_ids(&lib, View::Photos), [6, 3, 1]);
    }

    #[test]
    fn la_papelera_va_por_fecha_de_envio() {
        let (_dir, lib) = library();
        // Cada foto se envía en un momento distinto, en otro orden que el de
        // su fecha de captura.
        for (id, at) in [(2, 300), (3, 100), (4, 400), (5, 200)] {
            lib.conn
                .execute("UPDATE photos SET trashed_at = ?2 WHERE id = ?1", [id, at])
                .unwrap();
        }
        let page = lib.view_page(View::Trash, None, 100).unwrap();
        let got: Vec<(i64, i64)> = page.photos.iter().map(|p| (p.id, p.date)).collect();
        assert_eq!(got, [(4, 400), (2, 300), (5, 200), (3, 100)]);
        assert_eq!(months_total(&lib, View::Trash), 4);

        // Paginación por cursor con la fecha de envío.
        let first = lib.view_page(View::Trash, None, 2).unwrap();
        let ids: Vec<i64> = first.photos.iter().map(|p| p.id).collect();
        assert_eq!(ids, [4, 2]);
        let rest = lib.view_page(View::Trash, first.next, 2).unwrap();
        let ids: Vec<i64> = rest.photos.iter().map(|p| p.id).collect();
        assert_eq!(ids, [5, 3]);
        assert!(rest.next.is_none());

        // La selección sigue el mismo orden.
        let trash = Scope::View(View::Trash);
        assert_eq!(lib.select_all(trash).unwrap(), [4, 2, 5, 3]);
        let a = crate::timeline::TimelineCursor { date: 300, id: 2 };
        let b = crate::timeline::TimelineCursor { date: 100, id: 3 };
        assert_eq!(lib.select_range(trash, b, a).unwrap(), [2, 5, 3]);
        // Las cuatro se enviaron el 1-1-1970 en UTC; en hora local puede ser
        // el 31-12-1969, así que se pregunta por el día que da SQLite.
        let day: String = lib
            .conn
            .query_row(
                "SELECT strftime('%Y-%m-%d', 300, 'unixepoch', 'localtime')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(lib.select_day(trash, &day).unwrap(), [4, 2, 5, 3]);
    }

    #[test]
    fn las_vistas_usan_sus_indices_parciales() {
        let (_dir, lib) = library();
        for (view, index) in [
            (View::Photos, "photos_timeline_fotos"),
            (View::Favorites, "photos_timeline_favoritos"),
            (View::Archive, "photos_timeline_archivo"),
            (View::Trash, "photos_papelera"),
        ] {
            let plan: String = lib
                .conn
                .prepare(&format!(
                    "EXPLAIN QUERY PLAN SELECT id FROM photos WHERE 1 {}
                     ORDER BY {} DESC, id DESC",
                    view.filter().sql,
                    view.filter().order
                ))
                .unwrap()
                .query_map([], |r| r.get::<_, String>(3))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
                .join("\n");
            assert!(plan.contains(index), "{view:?}: {plan}");
        }
    }
}
