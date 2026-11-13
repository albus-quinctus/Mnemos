//! Favoritos y archivo (spec 006).
//!
//! Cada acción recibe la lista de ids de la selección y la aplica en una sola
//! transacción: o cambian todas las fotos o ninguna. Solo se toca la base de
//! datos; los archivos de imagen no se abren.
//!
//! Devuelven los ids que han cambiado de verdad, en el orden recibido y sin
//! repetir. La interfaz los usa para «Deshacer»: aplicar la acción contraria
//! solo a esas fotos deja la biblioteca como estaba, aunque la selección
//! mezclara fotos que ya eran favoritas o ya estaban archivadas.

use std::time::SystemTime;

use crate::error::Result;
use crate::files::unix_secs;
use crate::library::Library;

impl Library {
    /// Añade a Favoritos (`true`) o quita de Favoritos (`false`) las fotos
    /// `ids`. Los ids que no existen se ignoran.
    pub fn set_favorite(&mut self, ids: &[i64], favorite: bool) -> Result<Vec<i64>> {
        self.update_each(
            ids,
            "UPDATE photos SET favorite = ?2 WHERE id = ?1 AND favorite <> ?2",
            Some(i64::from(favorite)),
        )
    }

    /// Archiva (`true`) o desarchiva (`false`) las fotos `ids`. Una foto ya
    /// archivada conserva su fecha de archivo. Los ids que no existen se
    /// ignoran.
    pub fn set_archived(&mut self, ids: &[i64], archived: bool) -> Result<Vec<i64>> {
        if archived {
            self.update_each(
                ids,
                "UPDATE photos SET archived_at = ?2 WHERE id = ?1 AND archived_at IS NULL",
                Some(unix_secs(SystemTime::now())),
            )
        } else {
            self.update_each(
                ids,
                "UPDATE photos SET archived_at = NULL WHERE id = ?1 AND archived_at IS NOT NULL",
                None,
            )
        }
    }

    /// Ejecuta `sql` (con `?1` = id y, si hay `value`, `?2`) para cada id dentro de
    /// una transacción. Una fila por sentencia evita el límite de parámetros
    /// de SQLite con selecciones de decenas de miles de fotos.
    pub(crate) fn update_each(
        &mut self,
        ids: &[i64],
        sql: &str,
        value: Option<i64>,
    ) -> Result<Vec<i64>> {
        let tx = self.conn.transaction()?;
        let mut changed = Vec::new();
        {
            let mut stmt = tx.prepare(sql)?;
            for &id in ids {
                let rows = match value {
                    Some(value) => stmt.execute((id, value))?,
                    None => stmt.execute([id])?,
                };
                if rows > 0 {
                    changed.push(id);
                }
            }
        }
        tx.commit()?;
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn library(n: i64) -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        let tx = lib.conn.transaction().unwrap();
        for i in 1..=n {
            tx.execute(
                "INSERT INTO photos (id, rel_path, size_bytes, modified_at, imported_at)
                     VALUES (?1, ?2, 1, ?1, 0)",
                params![i, format!("Originales/{i}.jpg")],
            )
            .unwrap();
        }
        tx.commit().unwrap();
        (dir, lib)
    }

    fn ids_where(lib: &Library, condition: &str) -> Vec<i64> {
        lib.conn
            .prepare(&format!(
                "SELECT id FROM photos WHERE {condition} ORDER BY id"
            ))
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn marca_y_desmarca_solo_las_fotos_elegidas() {
        let (_dir, mut lib) = library(5);
        assert_eq!(lib.set_favorite(&[2, 4], true).unwrap(), [2, 4]);
        assert_eq!(ids_where(&lib, "favorite = 1"), [2, 4]);

        assert_eq!(lib.set_favorite(&[4], false).unwrap(), [4]);
        assert_eq!(ids_where(&lib, "favorite = 1"), [2]);
    }

    #[test]
    fn devuelve_solo_lo_que_cambia_para_poder_deshacer() {
        let (_dir, mut lib) = library(4);
        lib.set_favorite(&[1], true).unwrap();
        // Selección mixta: la 1 ya era favorita, la 99 no existe y la 3 va
        // repetida.
        let changed = lib.set_favorite(&[1, 3, 99, 3, 2], true).unwrap();
        assert_eq!(changed, [3, 2]);

        // Deshacer quita solo las que se añadieron: la 1 sigue siendo favorita.
        lib.set_favorite(&changed, false).unwrap();
        assert_eq!(ids_where(&lib, "favorite = 1"), [1]);
    }

    #[test]
    fn archiva_y_desarchiva_conservando_la_fecha_de_archivo() {
        let (_dir, mut lib) = library(3);
        assert_eq!(lib.set_archived(&[1, 2], true).unwrap(), [1, 2]);
        assert_eq!(ids_where(&lib, "archived_at IS NOT NULL"), [1, 2]);

        lib.conn
            .execute("UPDATE photos SET archived_at = 5 WHERE id = 1", [])
            .unwrap();
        assert_eq!(lib.set_archived(&[1, 3], true).unwrap(), [3]);
        let first: i64 = lib
            .conn
            .query_row("SELECT archived_at FROM photos WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(first, 5);

        assert_eq!(lib.set_archived(&[2, 3, 99], false).unwrap(), [2, 3]);
        assert_eq!(ids_where(&lib, "archived_at IS NOT NULL"), [1]);
    }

    #[test]
    fn favorito_y_archivo_son_independientes() {
        let (_dir, mut lib) = library(2);
        lib.set_favorite(&[1], true).unwrap();
        lib.set_archived(&[1], true).unwrap();
        lib.set_archived(&[1], false).unwrap();
        assert_eq!(ids_where(&lib, "favorite = 1"), [1]);
    }

    #[test]
    fn si_falla_a_mitad_no_cambia_ninguna_foto() {
        let (_dir, mut lib) = library(5);
        // Simula un fallo de escritura al llegar a la foto 4.
        lib.conn
            .execute_batch(
                "CREATE TRIGGER fallo BEFORE UPDATE ON photos WHEN NEW.id = 4
                 BEGIN SELECT RAISE(ABORT, 'disco desconectado'); END;",
            )
            .unwrap();
        assert!(lib.set_favorite(&[1, 2, 3, 4, 5], true).is_err());
        assert!(ids_where(&lib, "favorite = 1").is_empty());
        assert!(lib.set_archived(&[1, 2, 3, 4, 5], true).is_err());
        assert!(ids_where(&lib, "archived_at IS NOT NULL").is_empty());
    }

    #[test]
    fn selecciones_enormes_y_vacias() {
        let (_dir, mut lib) = library(40_000);
        let all: Vec<i64> = (1..=40_000).collect();
        assert_eq!(lib.set_favorite(&all, true).unwrap().len(), 40_000);
        assert_eq!(lib.set_archived(&all, true).unwrap().len(), 40_000);
        assert!(lib.set_favorite(&[], true).unwrap().is_empty());
    }

    #[test]
    fn no_toca_los_archivos() {
        let dir = tempfile::tempdir().unwrap();
        let originales = dir.path().join("Originales");
        std::fs::create_dir_all(&originales).unwrap();
        let foto = originales.join("a.jpg");
        std::fs::write(&foto, b"no es un jpeg de verdad").unwrap();
        let antes = std::fs::metadata(&foto).unwrap().modified().unwrap();

        let mut lib = Library::create(dir.path()).unwrap();
        lib.index().unwrap();
        let hash = |lib: &Library| -> Option<String> {
            lib.conn
                .query_row("SELECT content_hash FROM photos", [], |r| r.get(0))
                .unwrap()
        };
        let id: i64 = lib
            .conn
            .query_row("SELECT id FROM photos", [], |r| r.get(0))
            .unwrap();
        let huella = hash(&lib);

        lib.set_favorite(&[id], true).unwrap();
        lib.set_archived(&[id], true).unwrap();
        lib.set_archived(&[id], false).unwrap();
        lib.set_favorite(&[id], false).unwrap();

        assert_eq!(std::fs::read(&foto).unwrap(), b"no es un jpeg de verdad");
        assert_eq!(std::fs::metadata(&foto).unwrap().modified().unwrap(), antes);
        assert_eq!(crate::files::hash_file(&foto).ok(), huella);
    }
}
