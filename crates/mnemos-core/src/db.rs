//! Base de datos SQLite de la biblioteca y sus migraciones.

use std::path::Path;

use rusqlite::Connection;

use crate::error::{Error, Result};

/// Migraciones en orden. La posición `i` lleva el esquema a la versión `i + 1`.
/// Nunca se edita una migración publicada: se añade una nueva.
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/0001_inicial.sql"),
    include_str!("migrations/0002_orientacion.sql"),
    include_str!("migrations/0003_orientacion_raw.sql"),
    include_str!("migrations/0004_fallos_miniaturas.sql"),
    include_str!("migrations/0005_indice_cronologico.sql"),
    include_str!("migrations/0006_fecha_captura.sql"),
    include_str!("migrations/0007_favoritos_archivo.sql"),
    include_str!("migrations/0008_metadatos_takeout.sql"),
    include_str!("migrations/0011_fecha_manual.sql"),
    // Se llama 0009 porque se reservó ese número para la spec 009, pero se
    // fusionó después de la 0011 y por eso va al final.
    include_str!("migrations/0009_edicion.sql"),
    include_str!("migrations/0013_etiquetas.sql"),
    // Papelera (spec 015). La 0012 (álbumes) irá detrás cuando se fusione.
    include_str!("migrations/0014_papelera.sql"),
];

/// Versión de esquema que entiende esta versión de MNEMOS.
pub const SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

/// Abre (o crea) la base de datos y la lleva a la última versión del esquema.
pub fn open(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrate(&mut conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    // La importación escribe mientras la cuadrícula lee: esperar un poco es
    // mejor que fallar con "database is locked".
    conn.busy_timeout(std::time::Duration::from_secs(10))?;
    // Journal clásico y escritura síncrona completa: más seguro en discos
    // externos que se pueden desconectar y en sistemas de archivos como exFAT.
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = DELETE;
         PRAGMA synchronous = FULL;",
    )?;
    Ok(())
}

/// Versión actual del esquema guardada en la base de datos.
pub fn schema_version(conn: &Connection) -> Result<u32> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let current = schema_version(conn)?;
    if current > SCHEMA_VERSION {
        return Err(Error::SchemaTooNew {
            found: current,
            supported: SCHEMA_VERSION,
        });
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as u32 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crea_el_esquema_completo() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("mnemos.db")).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);

        for table in [
            "photos",
            "albums",
            "album_photos",
            "tags",
            "photo_tags",
            "photo_search",
            "day_titles",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name = ?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "falta la tabla {table}");
        }
    }

    #[test]
    fn reabrir_no_repite_migraciones() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mnemos.db");
        drop(open(&path).unwrap());
        let conn = open(&path).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn la_migracion_3_vuelve_a_leer_la_orientacion_de_ciertos_raw() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mnemos.db");
        {
            // Biblioteca en la versión 2, con la orientación ya rellenada.
            let mut conn = Connection::open(&path).unwrap();
            for (i, sql) in MIGRATIONS[..2].iter().enumerate() {
                let tx = conn.transaction().unwrap();
                tx.execute_batch(sql).unwrap();
                tx.pragma_update(None, "user_version", i as u32 + 1)
                    .unwrap();
                tx.commit().unwrap();
            }
            for name in ["a.CR3", "b.raf", "c.orf", "d.Rw2", "e.nef", "f.jpg"] {
                conn.execute(
                    "INSERT INTO photos (rel_path, size_bytes, modified_at, orientation, imported_at)
                     VALUES (?1, 1, 1, 1, 1)",
                    [name],
                )
                .unwrap();
            }
        }
        let conn = open(&path).unwrap();
        let pending: Vec<String> = conn
            .prepare("SELECT rel_path FROM photos WHERE orientation IS NULL ORDER BY rel_path")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(pending, ["a.CR3", "b.raf", "c.orf", "d.Rw2"]);
    }

    #[test]
    fn la_migracion_7_anade_favoritos_archivo_y_titulos_sin_tocar_las_fotos() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mnemos.db");
        {
            // Biblioteca en la versión 6 con una foto.
            let mut conn = Connection::open(&path).unwrap();
            for (i, sql) in MIGRATIONS[..6].iter().enumerate() {
                let tx = conn.transaction().unwrap();
                tx.execute_batch(sql).unwrap();
                tx.pragma_update(None, "user_version", i as u32 + 1)
                    .unwrap();
                tx.commit().unwrap();
            }
            conn.execute(
                "INSERT INTO photos (rel_path, size_bytes, modified_at, taken_at, imported_at)
                 VALUES ('Originales/2024/10/a.jpg', 1, 5, 7, 9)",
                [],
            )
            .unwrap();
        }
        let conn = open(&path).unwrap();
        let fila: (String, i64, Option<i64>, i64, i64) = conn
            .query_row(
                "SELECT rel_path, favorite, archived_at, modified_at, taken_at FROM photos",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(fila, ("Originales/2024/10/a.jpg".into(), 0, None, 5, 7));

        for index in [
            "photos_timeline_fotos",
            "photos_timeline_favoritos",
            "photos_timeline_archivo",
        ] {
            let n: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                    [index],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "falta el índice {index}");
        }
    }

    #[test]
    fn la_migracion_8_anade_los_metadatos_de_takeout_vacios() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mnemos.db");
        {
            // Biblioteca en la versión 7 con una foto favorita.
            let mut conn = Connection::open(&path).unwrap();
            for (i, sql) in MIGRATIONS[..7].iter().enumerate() {
                let tx = conn.transaction().unwrap();
                tx.execute_batch(sql).unwrap();
                tx.pragma_update(None, "user_version", i as u32 + 1)
                    .unwrap();
                tx.commit().unwrap();
            }
            conn.execute(
                "INSERT INTO photos (rel_path, size_bytes, modified_at, imported_at, favorite)
                 VALUES ('a.jpg', 1, 1, 1, 1)",
                [],
            )
            .unwrap();
        }
        let conn = open(&path).unwrap();
        let fila: (i64, Option<String>, Option<f64>, Option<i64>) = conn
            .query_row(
                "SELECT favorite, description, latitude, takeout_at FROM photos",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(fila, (1, None, None, None));
        // La ubicación tiene que estar en rango.
        assert!(conn.execute("UPDATE photos SET latitude = 91", []).is_err());
        assert!(
            conn.execute("UPDATE photos SET longitude = -181", [])
                .is_err()
        );
    }

    #[test]
    fn la_migracion_10_anade_la_version_de_edicion_vacia() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mnemos.db");
        {
            // Biblioteca en la versión 9 (con la fecha a mano) y una foto.
            let mut conn = Connection::open(&path).unwrap();
            for (i, sql) in MIGRATIONS[..9].iter().enumerate() {
                let tx = conn.transaction().unwrap();
                tx.execute_batch(sql).unwrap();
                tx.pragma_update(None, "user_version", i as u32 + 1)
                    .unwrap();
                tx.commit().unwrap();
            }
            conn.execute(
                "INSERT INTO photos (rel_path, size_bytes, modified_at, imported_at, taken_source)
                 VALUES ('a.jpg', 1, 1, 1, 'manual')",
                [],
            )
            .unwrap();
        }
        let conn = open(&path).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);
        let fila: (Option<i64>, String) = conn
            .query_row("SELECT edit_version, taken_source FROM photos", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(fila, (None, "manual".to_owned()));
        assert!(
            conn.execute("UPDATE photos SET edit_version = 0", [])
                .is_err()
        );
        conn.execute("UPDATE photos SET edit_version = 3", [])
            .unwrap();
    }

    #[test]
    fn la_migracion_de_etiquetas_rehace_su_columna_en_la_busqueda() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mnemos.db");
        let antes = MIGRATIONS
            .iter()
            .position(|m| m.contains("Etiquetas (spec 016)"))
            .unwrap();
        {
            // Biblioteca justo antes de la migración, con una etiqueta que
            // el índice de búsqueda no conoce y otra fila con texto de más.
            let mut conn = Connection::open(&path).unwrap();
            for (i, sql) in MIGRATIONS[..antes].iter().enumerate() {
                let tx = conn.transaction().unwrap();
                tx.execute_batch(sql).unwrap();
                tx.pragma_update(None, "user_version", i as u32 + 1)
                    .unwrap();
                tx.commit().unwrap();
            }
            conn.execute_batch(
                "PRAGMA foreign_keys = ON;
                 INSERT INTO photos (id, rel_path, size_bytes, modified_at, imported_at)
                     VALUES (1, 'a.jpg', 1, 1, 1), (2, 'b.jpg', 1, 1, 1), (3, 'c.jpg', 1, 1, 1);
                 INSERT INTO photo_search (rowid, rel_path, tags)
                     VALUES (1, 'a.jpg', ''), (2, 'b.jpg', 'vieja'), (3, 'c.jpg', '');
                 INSERT INTO tags (id, name) VALUES (1, 'Playa'), (2, 'Málaga');
                 INSERT INTO photo_tags (photo_id, tag_id) VALUES (1, 1), (1, 2);",
            )
            .unwrap();
        }
        let conn = open(&path).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);
        let tags = |id: i64| -> String {
            conn.query_row(
                "SELECT tags FROM photo_search WHERE rowid = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap()
        };
        let uno = tags(1);
        assert!(uno.contains("Playa") && uno.contains("Málaga"), "{uno}");
        assert_eq!(tags(2), "");
        assert_eq!(tags(3), "");
        let indice: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'photo_tags_tag'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(indice, 1);
    }

    #[test]
    fn favorito_solo_admite_cero_o_uno() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("mnemos.db")).unwrap();
        let res = conn.execute(
            "INSERT INTO photos (rel_path, size_bytes, modified_at, imported_at, favorite)
             VALUES ('a.jpg', 1, 1, 1, 2)",
            [],
        );
        assert!(res.is_err());
    }

    #[test]
    fn los_titulos_de_dia_validan_fecha_y_longitud() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("mnemos.db")).unwrap();
        let guardar = |day: &str, title: &str| {
            conn.execute(
                "INSERT INTO day_titles (day, title, updated_at) VALUES (?1, ?2, 1)",
                [day, title],
            )
        };
        assert!(guardar("2024-10-12", "Ruta del Cares 🥾").is_ok());
        assert!(guardar("2024-10-13", &"á".repeat(100)).is_ok());
        assert!(guardar("2024-10-14", &"a".repeat(101)).is_err());
        assert!(guardar("2024-10-15", "").is_err());
        assert!(guardar("12/10/2024", "x").is_err());
        assert!(guardar("2024-10-12", "repetido").is_err());
    }

    #[test]
    fn rechaza_esquemas_mas_nuevos() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mnemos.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
                .unwrap();
        }
        assert!(matches!(open(&path), Err(Error::SchemaTooNew { .. })));
    }

    #[test]
    fn la_busqueda_de_texto_completo_ignora_acentos() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("mnemos.db")).unwrap();
        conn.execute(
            "INSERT INTO photo_search (rowid, rel_path, tags) VALUES (1, 'Originales/2024/cumpleaños.jpg', 'Málaga playa')",
            [],
        )
        .unwrap();
        let hits: i64 = conn
            .query_row(
                "SELECT count(*) FROM photo_search WHERE photo_search MATCH 'malaga'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1);
    }

    #[test]
    fn las_claves_ajenas_estan_activas() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("mnemos.db")).unwrap();
        let res = conn.execute(
            "INSERT INTO photo_tags (photo_id, tag_id) VALUES (99, 99)",
            [],
        );
        assert!(res.is_err());
    }
    #[test]
    fn la_migracion_de_la_papelera_conserva_favoritos_y_archivo() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mnemos.db");
        {
            // Biblioteca justo antes de la papelera, con una favorita
            // archivada.
            let mut conn = Connection::open(&path).unwrap();
            let before = MIGRATIONS.len() - 1;
            for (i, sql) in MIGRATIONS[..before].iter().enumerate() {
                let tx = conn.transaction().unwrap();
                tx.execute_batch(sql).unwrap();
                tx.pragma_update(None, "user_version", i as u32 + 1)
                    .unwrap();
                tx.commit().unwrap();
            }
            conn.execute(
                "INSERT INTO photos (rel_path, size_bytes, modified_at, imported_at,
                                     favorite, archived_at)
                 VALUES ('a.jpg', 1, 1, 1, 1, 5)",
                [],
            )
            .unwrap();
        }
        let conn = open(&path).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);
        let fila: (i64, Option<i64>, Option<i64>) = conn
            .query_row(
                "SELECT favorite, archived_at, trashed_at FROM photos",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(fila, (1, Some(5), None));
        let meta: i64 = conn
            .query_row("SELECT count(*) FROM library_meta", [], |r| r.get(0))
            .unwrap();
        assert_eq!(meta, 0);
    }
}
