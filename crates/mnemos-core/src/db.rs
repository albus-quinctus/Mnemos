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
}
