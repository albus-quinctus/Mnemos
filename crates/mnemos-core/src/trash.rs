//! Papelera (spec 015).
//!
//! Enviar una foto a la Papelera solo pone la fecha en `photos.trashed_at`:
//! el archivo sigue en su sitio. Las vistas, la búsqueda y los duplicados
//! dejan fuera esas fotos.
//!
//! Enviar y restaurar funcionan como archivar (spec 006): una transacción
//! para toda la selección y, de vuelta, los ids que cambiaron de verdad para
//! poder deshacer.
//!
//! **Excepción a la regla 1 de `CLAUDE.md`:** este es el único módulo que
//! borra originales, siempre con [`delete_original`] y solo para fotos que
//! ya están en la Papelera. Lo llaman «Eliminar para siempre» y el vaciado.
//! La prueba `tests/solo_la_papelera_borra.rs` falla si otro código del
//! núcleo empieza a borrar archivos.

use std::fs;
use std::io;
use std::path::Path;
use std::time::SystemTime;

use rusqlite::OptionalExtension;
use serde::Serialize;

use crate::edits::recipe_rel_path;
use crate::error::{Error, Result};
use crate::files::unix_secs;
use crate::library::{Library, ORIGINALS_DIR};
use crate::paths::RelPath;
use crate::thumbnails::thumbnail_rel_path;

/// Días que pasa una foto en la Papelera antes de eliminarse para siempre.
pub const TRASH_DAYS: i64 = 30;

/// Si la fecha del equipo va más de estos días por delante de la última vez
/// que se abrió la biblioteca, el vaciado automático pregunta antes.
pub const CLOCK_AHEAD_DAYS: i64 = 60;

const DAY: i64 = 86_400;

/// Clave de `library_meta` con la última apertura (segundos Unix, UTC).
const LAST_OPEN: &str = "ultima_apertura";

/// Resultado del vaciado automático de la Papelera.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum AutoEmpty {
    /// Se eliminaron (o se intentaron eliminar) las fotos caducadas.
    Emptied(DeleteReport),
    /// La fecha del equipo parece adelantada: no se ha borrado nada. Hay
    /// que preguntar y, si el usuario acepta, volver a llamar con
    /// `trust_clock`.
    ClockAhead { expired: u64, last_open: i64 },
}

/// Resultado de eliminar fotos para siempre.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct DeleteReport {
    /// Fotos eliminadas: su archivo ya no está y su fila tampoco.
    pub deleted: Vec<i64>,
    /// Fotos que no se pudieron eliminar y siguen en la Papelera.
    pub failed: Vec<DeleteFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeleteFailure {
    pub id: i64,
    pub rel_path: String,
    /// Por qué no, en una frase para el aviso.
    pub reason: String,
}

/// Lo que la base de datos sabe del archivo de una foto de la Papelera.
struct Trashed {
    id: i64,
    rel_path: String,
    size: i64,
    modified: i64,
    hash: Option<String>,
}

/// Qué pasó con el archivo.
#[derive(Debug, PartialEq, Eq)]
enum Removed {
    Deleted,
    /// Ya no estaba: la foto era una desaparecida.
    AlreadyGone,
}

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

    /// Elimina para siempre las fotos `ids` que estén en la Papelera: borra
    /// su archivo y su fila. Las que no están en la Papelera y los ids que no
    /// existen se ignoran. `progress` recibe cuántas van y cuántas son.
    ///
    /// Una foto cuyo archivo cambió fuera de la app (otro tamaño u otra
    /// fecha), o que no se puede borrar, se queda en la Papelera y sale en
    /// `failed`. Si el disco deja de estar a mitad, para con un error: lo ya
    /// eliminado queda eliminado y el resto sigue en la Papelera.
    pub fn delete_permanently(
        &mut self,
        ids: &[i64],
        mut progress: impl FnMut(usize, usize),
    ) -> Result<DeleteReport> {
        self.check_disk()?;
        let photos = self.trashed_photos(ids)?;
        let total = photos.len();
        let mut report = DeleteReport::default();
        for (done, photo) in photos.into_iter().enumerate() {
            progress(done, total);
            match delete_original(&self.root, &photo.rel_path, photo.size, photo.modified) {
                Ok(Removed::Deleted) => {}
                // Antes de dar por desaparecida la foto, se comprueba que el
                // disco sigue ahí.
                Ok(Removed::AlreadyGone) => self.check_disk()?,
                Err(err) => {
                    // ¿Se ha desconectado el disco? Entonces no se sigue.
                    self.check_disk()?;
                    report.failed.push(DeleteFailure {
                        id: photo.id,
                        rel_path: photo.rel_path,
                        reason: err.to_string(),
                    });
                    continue;
                }
            }
            self.forget_photo(&photo)?;
            report.deleted.push(photo.id);
        }
        progress(total, total);
        Ok(report)
    }

    /// Vaciado automático (spec 015): elimina para siempre las fotos que
    /// llevan [`TRASH_DAYS`] días o más en la Papelera a fecha de `now`
    /// (segundos Unix) y guarda `now` como última apertura.
    ///
    /// Si `now` va más de [`CLOCK_AHEAD_DAYS`] días por delante de la última
    /// apertura y hay algo que eliminar, no borra nada ni guarda la fecha:
    /// devuelve [`AutoEmpty::ClockAhead`] para que la app pregunte. Con
    /// `trust_clock` (el usuario ha dicho que sí) se salta esa comprobación.
    pub fn empty_expired_trash(
        &mut self,
        now: i64,
        trust_clock: bool,
        progress: impl FnMut(usize, usize),
    ) -> Result<AutoEmpty> {
        let expired = self.expired_trash(now)?;
        let last_open = self.last_open()?;
        if let Some(last_open) = last_open
            && !trust_clock
            && !expired.is_empty()
            && now - last_open > CLOCK_AHEAD_DAYS * DAY
        {
            return Ok(AutoEmpty::ClockAhead {
                expired: expired.len() as u64,
                last_open,
            });
        }
        let report = if expired.is_empty() {
            DeleteReport::default()
        } else {
            self.delete_permanently(&expired, progress)?
        };
        // La última apertura nunca va hacia atrás: un equipo con la fecha
        // retrasada no debe desactivar la comprobación para los demás.
        if last_open.is_none_or(|last| now > last) {
            self.conn.execute(
                "INSERT INTO library_meta (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                (LAST_OPEN, now.to_string()),
            )?;
        }
        Ok(AutoEmpty::Emptied(report))
    }

    /// Fotos que llevan [`TRASH_DAYS`] días o más en la Papelera a fecha de
    /// `now`, de la más antigua a la más reciente.
    pub fn expired_trash(&self, now: i64) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM photos WHERE trashed_at IS NOT NULL AND trashed_at <= ?1
             ORDER BY trashed_at, id",
        )?;
        let ids = stmt
            .query_map([now - TRASH_DAYS * DAY], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(ids)
    }

    /// Todas las fotos de la Papelera («Vaciar papelera»), de la más antigua
    /// a la más reciente.
    pub fn trash_ids(&self) -> Result<Vec<i64>> {
        self.expired_trash(i64::MAX / 2)
    }

    fn last_open(&self) -> Result<Option<i64>> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM library_meta WHERE key = ?1",
                [LAST_OPEN],
                |r| r.get(0),
            )
            .optional()?;
        Ok(value.and_then(|v| v.parse().ok()))
    }

    /// Error claro si la carpeta de originales no está: sin ella, todas las
    /// fotos parecerían desaparecidas y se borrarían sus filas.
    fn check_disk(&self) -> Result<()> {
        if self.root.join(ORIGINALS_DIR).is_dir() {
            return Ok(());
        }
        Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no se encuentra {} en {}: ¿está conectado el disco?",
                ORIGINALS_DIR,
                self.root.display()
            ),
        )))
    }

    /// Las fotos de `ids` que están en la Papelera, en el orden recibido y
    /// sin repetir.
    fn trashed_photos(&self, ids: &[i64]) -> Result<Vec<Trashed>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, rel_path, size_bytes, modified_at, content_hash
             FROM photos WHERE id = ?1 AND trashed_at IS NOT NULL",
        )?;
        let mut seen = std::collections::HashSet::new();
        let mut photos = Vec::new();
        for &id in ids {
            if !seen.insert(id) {
                continue;
            }
            let photo = stmt
                .query_row([id], |r| {
                    Ok(Trashed {
                        id: r.get(0)?,
                        rel_path: r.get(1)?,
                        size: r.get(2)?,
                        modified: r.get(3)?,
                        hash: r.get(4)?,
                    })
                })
                .optional()?;
            photos.extend(photo);
        }
        Ok(photos)
    }

    /// Borra la fila de una foto ya sin archivo, con su entrada de búsqueda;
    /// sus álbumes y etiquetas se van con ella (`ON DELETE CASCADE`). Si
    /// ninguna otra foto tiene su huella, borra también su receta y sus
    /// miniaturas, que solo eran suyas.
    fn forget_photo(&mut self, photo: &Trashed) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM photo_search WHERE rowid = ?1", [photo.id])?;
        tx.execute("DELETE FROM photos WHERE id = ?1", [photo.id])?;
        let orphan = match &photo.hash {
            Some(hash) => {
                let others: i64 = tx.query_row(
                    "SELECT count(*) FROM photos WHERE content_hash = ?1",
                    [hash],
                    |r| r.get(0),
                )?;
                if others == 0 {
                    tx.execute(
                        "DELETE FROM thumbnail_failures WHERE content_hash = ?1",
                        [hash],
                    )?;
                }
                (others == 0).then_some(hash)
            }
            None => None,
        };
        tx.commit()?;
        if let Some(hash) = orphan {
            remove_hash_files(&self.root, hash);
        }
        Ok(())
    }
}

/// Borra el original de una foto de la Papelera. **Es la única función de
/// MNEMOS que borra un original** (excepción a la regla 1 de `CLAUDE.md`).
///
/// Solo borra si el archivo es un archivo normal dentro de la biblioteca
/// (sin salir por enlaces simbólicos) y su tamaño y su fecha de
/// modificación son los que guarda la base de datos. Si ya no estaba,
/// no es un error.
fn delete_original(root: &Path, rel_path: &str, size: i64, modified: i64) -> Result<Removed> {
    let rel = RelPath::new(rel_path)?;
    let path = rel.to_path(root);
    let meta = match fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Removed::AlreadyGone),
        Err(err) => return Err(err.into()),
    };
    if !meta.is_file() {
        return Err(refuse(rel_path, "no es un archivo normal"));
    }
    let real_root = root.canonicalize()?;
    if !path.canonicalize()?.starts_with(&real_root) {
        return Err(Error::OutsideLibrary(path));
    }
    if (meta.len() as i64, unix_secs(meta.modified()?)) != (size, modified) {
        return Err(refuse(
            rel_path,
            "el archivo ha cambiado fuera de MNEMOS; vuelve a indexar la biblioteca",
        ));
    }
    fs::remove_file(&path)?;
    Ok(Removed::Deleted)
}

fn refuse(rel_path: &str, why: &str) -> Error {
    Error::Io(io::Error::other(format!("no se borra {rel_path}: {why}")))
}

/// Borra la receta y todas las miniaturas (la del original y las editadas)
/// de una huella que ya no tiene ninguna foto. Son datos de `.mnemos/`, no
/// originales; un fallo no importa, como en las demás cachés.
fn remove_hash_files(root: &Path, hash: &str) {
    let _ = fs::remove_file(root.join(recipe_rel_path(hash)));
    let thumbnail = root.join(thumbnail_rel_path(hash));
    let _ = fs::remove_file(&thumbnail);
    let Some(dir) = thumbnail.parent() else {
        return;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let prefix = format!("{hash}-e");
    for entry in entries.flatten() {
        let name = entry.file_name();
        let edited = name
            .to_str()
            .and_then(|n| n.strip_prefix(&prefix))
            .and_then(|n| n.strip_suffix(".jpg"))
            .is_some_and(|v| v.parse::<u32>().is_ok());
        if edited {
            let _ = fs::remove_file(entry.path());
        }
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
    fn las_etiquetas_no_cuentan_la_papelera() {
        let (_dir, mut lib) = library();
        let tag = lib.create_tag("verano").unwrap();
        // La 6 y la 1 están fuera de la Papelera; la 2 y la 3, dentro.
        for id in [1, 2, 3, 6] {
            lib.conn
                .execute(
                    "INSERT INTO photo_tags (photo_id, tag_id) VALUES (?1, ?2)",
                    [id, tag.id],
                )
                .unwrap();
        }
        let tag = lib.tag(tag.id).unwrap();
        assert_eq!((tag.photos, tag.cover), (2, Some(6)));
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

#[cfg(test)]
mod borrado {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::index::tests::{set_mtime, write_png};
    use crate::thumbnails::thumbnail_rel_path_for;

    struct Setup {
        dir: tempfile::TempDir,
        lib: Library,
    }

    impl Setup {
        fn root(&self) -> &Path {
            self.dir.path()
        }

        fn file(&self, name: &str) -> PathBuf {
            self.root().join(ORIGINALS_DIR).join(name)
        }

        fn id(&self, name: &str) -> i64 {
            self.lib
                .conn
                .query_row(
                    "SELECT id FROM photos WHERE rel_path = ?1",
                    [format!("{ORIGINALS_DIR}/{name}")],
                    |r| r.get(0),
                )
                .unwrap()
        }

        fn hash(&self, name: &str) -> String {
            self.lib
                .conn
                .query_row(
                    "SELECT content_hash FROM photos WHERE id = ?1",
                    [self.id(name)],
                    |r| r.get(0),
                )
                .unwrap()
        }

        fn count(&self, sql: &str) -> i64 {
            self.lib.conn.query_row(sql, [], |r| r.get(0)).unwrap()
        }

        fn delete(&mut self, ids: &[i64]) -> DeleteReport {
            self.lib.delete_permanently(ids, |_, _| {}).unwrap()
        }
    }

    /// «a.png», «b.png» y «copia de a.png» (los mismos bytes que «a.png»),
    /// indexadas. Ninguna está en la Papelera.
    fn setup() -> Setup {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        let originals = dir.path().join(ORIGINALS_DIR);
        write_png(&originals.join("a.png"), 10, 10, b"a");
        write_png(&originals.join("b.png"), 10, 10, b"b");
        write_png(&originals.join("copia de a.png"), 10, 10, b"a");
        for name in ["a.png", "b.png", "copia de a.png"] {
            set_mtime(&originals.join(name), 1_700_000_000);
        }
        lib.index().unwrap();
        Setup { dir, lib }
    }

    #[test]
    fn solo_borra_fotos_de_la_papelera() {
        let mut s = setup();
        let (a, b) = (s.id("a.png"), s.id("b.png"));
        s.lib.send_to_trash(&[a]).unwrap();
        let report = s.delete(&[a, b, 999, a]);
        assert_eq!(report.deleted, [a]);
        assert!(report.failed.is_empty());
        assert!(!s.file("a.png").exists());
        // La b no estaba en la Papelera: ni archivo ni fila se tocan.
        assert!(s.file("b.png").exists());
        assert!(s.file("copia de a.png").exists());
        assert_eq!(s.count("SELECT count(*) FROM photos"), 2);
        assert_eq!(s.count("SELECT count(*) FROM photo_search"), 2);
    }

    #[test]
    fn se_lleva_sus_etiquetas_y_su_entrada_de_busqueda() {
        let mut s = setup();
        let b = s.id("b.png");
        let tag = s.lib.create_tag("verano").unwrap();
        s.lib
            .conn
            .execute(
                "INSERT INTO photo_tags (photo_id, tag_id) VALUES (?1, ?2)",
                [b, tag.id],
            )
            .unwrap();
        s.lib.send_to_trash(&[b]).unwrap();
        s.delete(&[b]);
        assert_eq!(s.count("SELECT count(*) FROM photo_tags"), 0);
        assert_eq!(
            s.count("SELECT count(*) FROM photo_search WHERE rel_path LIKE '%b.png'"),
            0
        );
        // La etiqueta sigue existiendo, vacía.
        assert_eq!(s.lib.tag(tag.id).unwrap().photos, 0);
    }

    #[test]
    fn no_borra_un_archivo_cambiado_fuera_de_la_app() {
        let mut s = setup();
        let (a, b) = (s.id("a.png"), s.id("b.png"));
        s.lib.send_to_trash(&[a, b]).unwrap();
        // Otro tamaño para la a y otra fecha para la b.
        write_png(&s.file("a.png"), 10, 10, b"otra foto distinta");
        set_mtime(&s.file("a.png"), 1_700_000_000);
        set_mtime(&s.file("b.png"), 1_700_000_999);
        let report = s.delete(&[a, b]);
        assert!(report.deleted.is_empty());
        let failed: Vec<i64> = report.failed.iter().map(|f| f.id).collect();
        assert_eq!(failed, [a, b]);
        assert!(report.failed[0].reason.contains("ha cambiado"));
        assert!(s.file("a.png").exists() && s.file("b.png").exists());
        // Siguen en la Papelera.
        assert_eq!(
            s.count("SELECT count(*) FROM photos WHERE trashed_at IS NOT NULL"),
            2
        );
    }

    #[test]
    fn una_desaparecida_solo_pierde_su_fila() {
        let mut s = setup();
        let b = s.id("b.png");
        s.lib.send_to_trash(&[b]).unwrap();
        fs::remove_file(s.file("b.png")).unwrap();
        assert_eq!(s.delete(&[b]).deleted, [b]);
        assert_eq!(s.count("SELECT count(*) FROM photos"), 2);
    }

    #[test]
    fn con_el_disco_desconectado_no_borra_ninguna_fila() {
        let mut s = setup();
        let b = s.id("b.png");
        s.lib.send_to_trash(&[b]).unwrap();
        fs::rename(s.root().join(ORIGINALS_DIR), s.root().join("desconectado")).unwrap();
        let err = s.lib.delete_permanently(&[b], |_, _| {}).unwrap_err();
        assert!(
            err.to_string().contains("¿está conectado el disco?"),
            "{err}"
        );
        assert_eq!(s.count("SELECT count(*) FROM photos"), 3);
    }

    #[cfg(unix)]
    #[test]
    fn no_sigue_enlaces_simbolicos() {
        let mut s = setup();
        let fuera = tempfile::tempdir().unwrap();
        let victima = fuera.path().join("fuera.png");
        write_png(&victima, 10, 10, b"fuera");
        // Una foto que en la base de datos es un enlace a un archivo de fuera.
        std::os::unix::fs::symlink(&victima, s.file("enlace.png")).unwrap();
        let meta = fs::symlink_metadata(s.file("enlace.png")).unwrap();
        s.lib
            .conn
            .execute(
                "INSERT INTO photos (id, rel_path, size_bytes, modified_at, imported_at, trashed_at)
                 VALUES (50, 'Originales/enlace.png', ?1, ?2, 0, 1)",
                params_of(&meta),
            )
            .unwrap();
        let report = s.delete(&[50]);
        assert!(report.deleted.is_empty());
        assert!(victima.exists());
        assert!(fs::symlink_metadata(s.file("enlace.png")).is_ok());
    }

    #[cfg(unix)]
    fn params_of(meta: &fs::Metadata) -> [i64; 2] {
        [meta.len() as i64, unix_secs(meta.modified().unwrap())]
    }

    #[test]
    fn la_receta_y_las_miniaturas_se_van_con_la_ultima_copia() {
        let mut s = setup();
        let (a, copia) = (s.id("a.png"), s.id("copia de a.png"));
        let hash = s.hash("a.png");
        let recipe = s.root().join(recipe_rel_path(&hash));
        let thumbs = [
            s.root().join(thumbnail_rel_path(&hash)),
            s.root().join(thumbnail_rel_path_for(&hash, Some(1))),
            s.root().join(thumbnail_rel_path_for(&hash, Some(2))),
        ];
        for path in thumbs.iter().chain([&recipe]) {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"x").unwrap();
        }
        // La miniatura de otra huella en la misma carpeta no se toca.
        let other = s.root().join(thumbnail_rel_path(&s.hash("b.png")));
        fs::create_dir_all(other.parent().unwrap()).unwrap();
        fs::write(&other, b"x").unwrap();

        // Con la copia todavía en la biblioteca, todo se queda.
        s.lib.send_to_trash(&[a]).unwrap();
        s.delete(&[a]);
        assert!(recipe.exists());
        assert!(thumbs.iter().all(|t| t.exists()));

        // Al eliminar la última foto con esa huella, se van.
        s.lib.send_to_trash(&[copia]).unwrap();
        s.delete(&[copia]);
        assert!(!recipe.exists());
        assert!(thumbs.iter().all(|t| !t.exists()));
        assert!(other.exists());
    }

    #[test]
    fn informa_del_progreso() {
        let mut s = setup();
        let ids = [s.id("a.png"), s.id("b.png")];
        s.lib.send_to_trash(&ids).unwrap();
        let mut steps = Vec::new();
        s.lib
            .delete_permanently(&ids, |done, total| steps.push((done, total)))
            .unwrap();
        assert_eq!(steps, [(0, 2), (1, 2), (2, 2)]);
    }
}

#[cfg(test)]
mod vaciado {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::index::tests::{set_mtime, write_png};

    const HOY: i64 = 1_800_000_000;

    /// Cuatro fotos: «vieja.png» enviada hace 31 días, «justa.png» hace
    /// exactamente 30, «reciente.png» hace 29 y «fuera.png» sin enviar.
    fn setup() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        let originals = dir.path().join(ORIGINALS_DIR);
        for name in ["vieja.png", "justa.png", "reciente.png", "fuera.png"] {
            write_png(&originals.join(name), 10, 10, name.as_bytes());
            set_mtime(&originals.join(name), 1_700_000_000);
        }
        lib.index().unwrap();
        for (name, days) in [("vieja", 31), ("justa", 30), ("reciente", 29)] {
            lib.conn
                .execute(
                    "UPDATE photos SET trashed_at = ?2 WHERE rel_path = ?1",
                    (format!("{ORIGINALS_DIR}/{name}.png"), HOY - days * DAY),
                )
                .unwrap();
        }
        (dir, lib)
    }

    fn file(dir: &tempfile::TempDir, name: &str) -> PathBuf {
        dir.path().join(ORIGINALS_DIR).join(name)
    }

    fn set_last_open(lib: &Library, at: i64) {
        lib.conn
            .execute(
                "INSERT OR REPLACE INTO library_meta (key, value) VALUES (?1, ?2)",
                (LAST_OPEN, at.to_string()),
            )
            .unwrap();
    }

    fn emptied(result: AutoEmpty) -> DeleteReport {
        match result {
            AutoEmpty::Emptied(report) => report,
            other => panic!("se esperaba vaciado: {other:?}"),
        }
    }

    #[test]
    fn elimina_solo_lo_que_lleva_30_dias_o_mas() {
        let (dir, mut lib) = setup();
        set_last_open(&lib, HOY - DAY);
        let report = emptied(lib.empty_expired_trash(HOY, false, |_, _| {}).unwrap());
        assert_eq!(report.deleted.len(), 2);
        assert!(!file(&dir, "vieja.png").exists());
        assert!(!file(&dir, "justa.png").exists());
        assert!(file(&dir, "reciente.png").exists());
        assert!(file(&dir, "fuera.png").exists());
        assert_eq!(lib.last_open().unwrap(), Some(HOY));
    }

    #[test]
    fn con_el_reloj_adelantado_pregunta_y_no_borra() {
        let (dir, mut lib) = setup();
        set_last_open(&lib, HOY - 61 * DAY);
        let result = lib.empty_expired_trash(HOY, false, |_, _| {}).unwrap();
        assert_eq!(
            result,
            AutoEmpty::ClockAhead {
                expired: 2,
                last_open: HOY - 61 * DAY
            }
        );
        assert!(file(&dir, "vieja.png").exists());
        // No guarda la fecha dudosa: la próxima vez vuelve a preguntar.
        assert_eq!(lib.last_open().unwrap(), Some(HOY - 61 * DAY));
        assert!(matches!(
            lib.empty_expired_trash(HOY, false, |_, _| {}).unwrap(),
            AutoEmpty::ClockAhead { .. }
        ));

        // El usuario dice que la fecha está bien.
        let report = emptied(lib.empty_expired_trash(HOY, true, |_, _| {}).unwrap());
        assert_eq!(report.deleted.len(), 2);
        assert_eq!(lib.last_open().unwrap(), Some(HOY));
    }

    #[test]
    fn sin_nada_caducado_no_pregunta_aunque_el_reloj_salte() {
        let (_dir, mut lib) = setup();
        lib.conn
            .execute("UPDATE photos SET trashed_at = NULL", [])
            .unwrap();
        set_last_open(&lib, HOY - 400 * DAY);
        let report = emptied(lib.empty_expired_trash(HOY, false, |_, _| {}).unwrap());
        assert!(report.deleted.is_empty());
        assert_eq!(lib.last_open().unwrap(), Some(HOY));
    }

    #[test]
    fn la_primera_apertura_vacia_sin_preguntar() {
        let (_dir, mut lib) = setup();
        assert_eq!(lib.last_open().unwrap(), None);
        let report = emptied(lib.empty_expired_trash(HOY, false, |_, _| {}).unwrap());
        assert_eq!(report.deleted.len(), 2);
    }

    #[test]
    fn un_reloj_retrasado_no_borra_ni_hace_retroceder_la_fecha() {
        let (dir, mut lib) = setup();
        set_last_open(&lib, HOY);
        let report = emptied(
            lib.empty_expired_trash(HOY - 40 * DAY, false, |_, _| {})
                .unwrap(),
        );
        assert!(report.deleted.is_empty());
        assert!(file(&dir, "vieja.png").exists());
        assert_eq!(lib.last_open().unwrap(), Some(HOY));
    }

    #[test]
    fn vaciar_la_papelera_lista_todas_de_la_mas_antigua_a_la_mas_reciente() {
        let (_dir, lib) = setup();
        let names: Vec<String> = lib
            .trash_ids()
            .unwrap()
            .into_iter()
            .map(|id| {
                lib.conn
                    .query_row("SELECT rel_path FROM photos WHERE id = ?1", [id], |r| {
                        r.get(0)
                    })
                    .unwrap()
            })
            .collect();
        assert_eq!(
            names,
            [
                "Originales/vieja.png",
                "Originales/justa.png",
                "Originales/reciente.png"
            ]
        );
    }

    #[test]
    fn importar_una_foto_de_la_papelera_no_la_trae_otra_vez() {
        let (dir, mut lib) = setup();
        let origen = tempfile::tempdir().unwrap();
        fs::copy(file(&dir, "vieja.png"), origen.path().join("vieja.png")).unwrap();
        fs::copy(file(&dir, "fuera.png"), origen.path().join("fuera.png")).unwrap();
        write_png(&origen.path().join("nueva.png"), 10, 10, b"nueva");
        let report = lib.import_from(origen.path()).unwrap();
        assert_eq!(
            (report.copied, report.duplicates, report.in_trash),
            (1, 1, 1)
        );
    }
}
