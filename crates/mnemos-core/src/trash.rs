//! Papelera (spec 015).
//!
//! Enviar una foto a la Papelera solo pone la fecha en `photos.trashed_at`:
//! el archivo sigue en su sitio. Las vistas, la búsqueda y los duplicados
//! dejan fuera esas fotos. Borrar el archivo es la única excepción a la regla
//! 1 de `CLAUDE.md` y vivirá solo en este módulo.

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
    fn las_vistas_usan_sus_indices_parciales() {
        let (_dir, lib) = library();
        for (view, index) in [
            (View::Photos, "photos_timeline_fotos"),
            (View::Favorites, "photos_timeline_favoritos"),
            (View::Archive, "photos_timeline_archivo"),
        ] {
            let plan: String = lib
                .conn
                .prepare(&format!(
                    "EXPLAIN QUERY PLAN SELECT id FROM photos WHERE 1 {}
                     ORDER BY coalesce(taken_at, modified_at) DESC, id DESC",
                    view.filter().sql
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
