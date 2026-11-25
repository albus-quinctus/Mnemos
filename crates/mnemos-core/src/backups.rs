//! Copias automáticas de la base de datos (spec 022).
//!
//! `mnemos.db` guarda lo que no está en los originales: álbumes, etiquetas,
//! favoritos, fechas a mano, ediciones a la vista. Se copia entera, con la API
//! de copias de SQLite, en `.mnemos/copias/`:
//!
//! - una vez al día como mucho, al abrir la biblioteca: `mnemos-AAAA-MM-DD.db`;
//! - siempre antes de aplicar una migración:
//!   `mnemos-AAAA-MM-DD-hhmmss-antes-vNN.db`, con la versión copiada.
//!
//! La copia se escribe en `.mnemos/tmp/`, se comprueba con
//! `PRAGMA quick_check` y solo entonces se renombra a su sitio: nunca queda
//! una copia a medias con nombre de copia buena. Los originales no se tocan.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, MAIN_DB, OpenFlags};

use crate::error::{Error, Result};
use crate::library::{DATA_DIR, Library, TMP_DIR};

/// Carpeta de las copias dentro de `.mnemos/`.
pub const BACKUP_DIR: &str = "copias";
/// Prefijo de los archivos temporales de copia en `.mnemos/tmp/`.
const TMP_PREFIX: &str = "copia-";

/// Momento local del equipo, `("2026-11-24", "103205")`.
fn now_local(conn: &Connection) -> Result<(String, String)> {
    Ok(conn.query_row(
        "SELECT date('now', 'localtime'), strftime('%H%M%S', 'now', 'localtime')",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?)
}

/// Comprueba con `PRAGMA quick_check` la base de `conn`.
pub(crate) fn quick_check(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare("PRAGMA quick_check")?;
    let rows: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows == ["ok"])
}

/// Copia la base de `conn` en `data_dir/copias/name` pasando por
/// `data_dir/tmp/`. Devuelve la ruta de la copia.
pub(crate) fn write_backup(conn: &Connection, data_dir: &Path, name: &str) -> Result<PathBuf> {
    let tmp_dir = data_dir.join(TMP_DIR);
    let dir = data_dir.join(BACKUP_DIR);
    fs::create_dir_all(&tmp_dir)?;
    fs::create_dir_all(&dir)?;
    let tmp = tmp_dir.join(format!("{TMP_PREFIX}{name}"));
    let _ = fs::remove_file(&tmp);
    let result = (|| -> Result<PathBuf> {
        conn.backup(MAIN_DB, &tmp, None::<fn(rusqlite::backup::Progress)>)?;
        let ok = {
            let copy = Connection::open_with_flags(&tmp, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            quick_check(&copy)?
        };
        if !ok {
            return Err(Error::BackupDamaged(tmp.clone()));
        }
        // La copia tiene que estar en el disco antes de llevar su nombre.
        fs::File::open(&tmp)?.sync_all()?;
        let dest = dir.join(name);
        fs::rename(&tmp, &dest)?;
        Ok(dest)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Copia de antes de migrar la base de `db_path` desde la versión `version`.
pub(crate) fn backup_before_migration(
    conn: &Connection,
    db_path: &Path,
    version: u32,
) -> Result<PathBuf> {
    let data_dir = db_path.parent().unwrap_or(Path::new("."));
    let (day, time) = now_local(conn)?;
    write_backup(
        conn,
        data_dir,
        &format!("mnemos-{day}-{time}-antes-v{version}.db"),
    )
}

/// Borra las copias a medias que dejó una copia interrumpida (disco
/// desconectado, programa cerrado). Solo toca `.mnemos/tmp/copia-*`.
fn clean_partial(data_dir: &Path) {
    let Ok(entries) = fs::read_dir(data_dir.join(TMP_DIR)) else {
        return;
    };
    for entry in entries.flatten() {
        if entry
            .file_name()
            .to_str()
            .is_some_and(|n| n.starts_with(TMP_PREFIX))
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

impl Library {
    fn data_dir(&self) -> PathBuf {
        self.root.join(DATA_DIR)
    }

    /// La copia diaria (spec 022): si aún no hay copia de hoy, la hace.
    /// Devuelve su ruta, o `None` si ya existía. Antes limpia las copias a
    /// medias de una vez anterior.
    pub fn daily_backup(&self) -> Result<Option<PathBuf>> {
        let data_dir = self.data_dir();
        clean_partial(&data_dir);
        let (day, _) = now_local(&self.conn)?;
        let name = format!("mnemos-{day}.db");
        if data_dir.join(BACKUP_DIR).join(&name).exists() {
            return Ok(None);
        }
        write_backup(&self.conn, &data_dir, &name).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nombres(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir)
            .map(|e| {
                e.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }

    #[test]
    fn una_copia_diaria_como_mucho() {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        lib.create_album("Viaje").unwrap();

        let copia = lib.daily_backup().unwrap().unwrap();
        let (day, _) = now_local(&lib.conn).unwrap();
        assert_eq!(
            copia.file_name().unwrap().to_str(),
            Some(format!("mnemos-{day}.db").as_str())
        );
        // La copia es una base completa y buena, con los mismos datos.
        let c = Connection::open(&copia).unwrap();
        assert!(quick_check(&c).unwrap());
        let albums: i64 = c
            .query_row("SELECT count(*) FROM albums", [], |r| r.get(0))
            .unwrap();
        assert_eq!(albums, 1);

        assert_eq!(lib.daily_backup().unwrap(), None);
        let copias = dir.path().join(DATA_DIR).join(BACKUP_DIR);
        assert_eq!(nombres(&copias).len(), 1);
        assert!(nombres(&dir.path().join(DATA_DIR).join(TMP_DIR)).is_empty());
    }

    #[test]
    fn limpia_las_copias_a_medias() {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        let tmp = dir.path().join(DATA_DIR).join(TMP_DIR);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("copia-mnemos-2020-01-01.db"), b"a medias").unwrap();
        fs::write(tmp.join("otra-cosa"), b"no es mia").unwrap();
        lib.daily_backup().unwrap();
        assert_eq!(nombres(&tmp), ["otra-cosa"]);
    }

    #[test]
    fn copia_antes_de_migrar() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join(DATA_DIR);
        fs::create_dir_all(&data).unwrap();
        let db_path = data.join("mnemos.db");
        {
            let mut conn = Connection::open(&db_path).unwrap();
            let tx = conn.transaction().unwrap();
            tx.execute_batch(crate::db::FIRST_MIGRATION).unwrap();
            tx.pragma_update(None, "user_version", 1).unwrap();
            tx.commit().unwrap();
        }
        let conn = crate::db::open(&db_path).unwrap();
        assert_eq!(
            crate::db::schema_version(&conn).unwrap(),
            crate::db::SCHEMA_VERSION
        );
        let copias = nombres(&data.join(BACKUP_DIR));
        assert_eq!(copias.len(), 1, "{copias:?}");
        assert!(copias[0].ends_with("-antes-v1.db"), "{copias:?}");
        let copia = Connection::open(data.join(BACKUP_DIR).join(&copias[0])).unwrap();
        assert_eq!(crate::db::schema_version(&copia).unwrap(), 1);

        // Abrirla otra vez, ya al día, no copia nada.
        drop(conn);
        crate::db::open(&db_path).unwrap();
        assert_eq!(nombres(&data.join(BACKUP_DIR)).len(), 1);
    }
}
