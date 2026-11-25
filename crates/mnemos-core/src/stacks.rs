//! Pilas (spec 023): varias fotos que son la misma toma (RAW + JPEG, una
//! ráfaga, una foto y su versión editada) se ven como una sola tarjeta.
//!
//! Una pila solo existe en la base de datos: apilar, desapilar o cambiar la
//! portada nunca abre, mueve, renombra ni borra un archivo.
//!
//! Invariantes que cada operación deja cumplidas, en la misma transacción:
//! - una pila tiene al menos 2 fotos (si se queda con una, deja de existir);
//! - su portada (`stacks.cover_id`) es una de sus fotos;
//! - `photos.stack_hidden` vale 1 justo en las fotos apiladas que no son la
//!   portada, para que las vistas las escondan sin unir tablas.

use std::collections::HashSet;
use std::time::SystemTime;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::files::unix_secs;
use crate::library::Library;
use crate::timeline::DATE;

/// De dónde salió una pila. Se enseña en la tarjeta y en Detalles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StackKind {
    /// Un RAW y su JPEG (u otra foto que no es RAW) con el mismo nombre.
    #[serde(rename = "raw_jpeg")]
    RawJpeg,
    /// Una foto y su versión editada (Google Fotos o «Guardar como copia»).
    #[serde(rename = "editada")]
    Edited,
    /// Una ráfaga, por el nombre o por la cámara, el segundo y el número.
    #[serde(rename = "rafaga")]
    Burst,
    /// Apilada a mano desde la barra de selección.
    #[serde(rename = "manual")]
    Manual,
}

impl StackKind {
    pub(crate) fn as_sql(self) -> &'static str {
        match self {
            StackKind::RawJpeg => "raw_jpeg",
            StackKind::Edited => "editada",
            StackKind::Burst => "rafaga",
            StackKind::Manual => "manual",
        }
    }

    fn from_sql(text: &str) -> rusqlite::Result<Self> {
        Ok(match text {
            "raw_jpeg" => StackKind::RawJpeg,
            "editada" => StackKind::Edited,
            "rafaga" => StackKind::Burst,
            "manual" => StackKind::Manual,
            other => {
                return Err(rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    format!("tipo de pila desconocido: {other}").into(),
                ));
            }
        })
    }
}

/// Lo que hace falta para deshacer un cambio de pilas: cómo estaban las pilas
/// y las fotos que tocó. La interfaz lo guarda tal cual y lo devuelve a
/// [`Library::restore_stacks`]; no necesita entenderlo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackUndo {
    /// Pilas que había antes y que el cambio tocó.
    stacks: Vec<SavedStack>,
    /// Pilas que creó el cambio: deshacer las quita.
    created: Vec<i64>,
    /// Cómo estaba cada foto tocada.
    photos: Vec<SavedPhoto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SavedStack {
    id: i64,
    cover_id: i64,
    kind: StackKind,
    created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SavedPhoto {
    id: i64,
    stack_id: Option<i64>,
    stack_manual: bool,
}

impl Library {
    /// Junta en una sola pila («Pila», hecha a mano) las fotos `ids` y las
    /// pilas en las que estén. La portada es la de la primera pila en el orden
    /// de la cuadrícula o, si no hay ninguna, la primera foto. Las fotos tienen
    /// que estar todas en la línea de tiempo o todas archivadas, y ninguna en
    /// la Papelera. Las fotos que entran quedan fuera de la agrupación
    /// automática: el usuario ya ha decidido.
    pub fn stack_photos(&mut self, ids: &[i64]) -> Result<StackUndo> {
        let tx = self.conn.transaction()?;
        for &id in ids {
            if !photo_exists(&tx, id)? {
                return Err(Error::PhotoNotFound(id));
            }
        }
        let members = expand(&tx, ids)?;
        if members.len() < 2 {
            return Err(Error::StackTooSmall);
        }
        check_same_place(&tx, &members)?;
        let cover = first_cover(&tx, &members)?;
        let undo_before = snapshot(&tx, &members)?;

        let old: HashSet<i64> = undo_before.stacks.iter().map(|s| s.id).collect();
        for id in &old {
            tx.execute("DELETE FROM stacks WHERE id = ?1", [id])?;
        }
        tx.execute(
            "INSERT INTO stacks (cover_id, kind, created_at) VALUES (?1, ?2, ?3)",
            params![
                cover,
                StackKind::Manual.as_sql(),
                unix_secs(SystemTime::now())
            ],
        )?;
        let stack = tx.last_insert_rowid();
        {
            let mut put = tx.prepare(
                "UPDATE photos SET stack_id = ?2, stack_hidden = (id <> ?3), stack_manual = 1
                 WHERE id = ?1",
            )?;
            for &id in &members {
                put.execute(params![id, stack, cover])?;
            }
        }
        refresh(&tx, stack)?;
        tx.commit()?;
        Ok(StackUndo {
            created: vec![stack],
            ..undo_before
        })
    }

    /// Deshace las pilas `stack_ids`: sus fotos vuelven a ir sueltas y la
    /// agrupación automática ya no las vuelve a apilar.
    pub fn unstack(&mut self, stack_ids: &[i64]) -> Result<StackUndo> {
        let tx = self.conn.transaction()?;
        let mut members = Vec::new();
        for &stack in stack_ids {
            if !stack_exists(&tx, stack)? {
                return Err(Error::StackNotFound(stack));
            }
            members.extend(members_of(&tx, stack)?);
        }
        let undo = snapshot(&tx, &members)?;
        for &stack in stack_ids {
            tx.execute(
                "UPDATE photos SET stack_id = NULL, stack_hidden = 0, stack_manual = 1
                 WHERE stack_id = ?1",
                [stack],
            )?;
            tx.execute("DELETE FROM stacks WHERE id = ?1", [stack])?;
        }
        tx.commit()?;
        Ok(undo)
    }

    /// Pone `photo` como portada de su pila.
    pub fn set_stack_cover(&mut self, photo: i64) -> Result<StackUndo> {
        let tx = self.conn.transaction()?;
        let stack = stack_of(&tx, photo)?.ok_or(Error::NotInStack(photo))?;
        let undo = snapshot(&tx, &members_of(&tx, stack)?)?;
        tx.execute(
            "UPDATE stacks SET cover_id = ?2 WHERE id = ?1",
            params![stack, photo],
        )?;
        refresh(&tx, stack)?;
        tx.commit()?;
        Ok(undo)
    }

    /// Deja las pilas y las fotos que tocó un cambio como estaban antes de él.
    pub fn restore_stacks(&mut self, undo: &StackUndo) -> Result<()> {
        let tx = self.conn.transaction()?;
        for &stack in &undo.created {
            tx.execute(
                "UPDATE photos SET stack_id = NULL, stack_hidden = 0 WHERE stack_id = ?1",
                [stack],
            )?;
            tx.execute("DELETE FROM stacks WHERE id = ?1", [stack])?;
        }
        for s in &undo.stacks {
            let updated = tx.execute(
                "UPDATE stacks SET cover_id = ?2, kind = ?3, created_at = ?4 WHERE id = ?1",
                params![s.id, s.cover_id, s.kind.as_sql(), s.created_at],
            )?;
            if updated == 0 {
                tx.execute(
                    "INSERT INTO stacks (id, cover_id, kind, created_at) VALUES (?1, ?2, ?3, ?4)",
                    params![s.id, s.cover_id, s.kind.as_sql(), s.created_at],
                )?;
            }
        }
        // Las pilas en las que están ahora las fotos, para rehacerlas después.
        let mut touched: HashSet<i64> = undo.stacks.iter().map(|s| s.id).collect();
        {
            let mut put = tx.prepare(
                "UPDATE photos SET stack_id = ?2, stack_hidden = 0, stack_manual = ?3
                 WHERE id = ?1",
            )?;
            for p in &undo.photos {
                if let Some(now) = stack_of(&tx, p.id)? {
                    touched.insert(now);
                }
                put.execute(params![p.id, p.stack_id, p.stack_manual])?;
            }
        }
        for stack in touched {
            if stack_exists(&tx, stack)? {
                refresh(&tx, stack)?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Fotos de la pila `stack`: la portada primero y luego por fecha, de la
    /// más antigua a la más reciente (el orden de una ráfaga).
    pub fn stack_members(&self, stack: i64) -> Result<Vec<i64>> {
        if !stack_exists(&self.conn, stack)? {
            return Err(Error::StackNotFound(stack));
        }
        members_of(&self.conn, stack)
    }

    /// Las fotos `ids` con todas las de sus pilas: lo que tocan las acciones
    /// sobre tarjetas de la cuadrícula. Conserva el orden y no repite.
    pub fn expand_stacks(&self, ids: &[i64]) -> Result<Vec<i64>> {
        expand(&self.conn, ids)
    }
}

/// Saca `photo` de su pila (al archivarla o tirarla sola, spec 023) y rehace
/// la pila: si era la portada, pasa a serlo la siguiente; si solo queda una
/// foto, la pila desaparece. La foto queda fuera de la agrupación automática.
/// No hace nada si la foto va suelta.
// Lo usarán archivar y tirar una sola foto desde la pila (tarea 6).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn remove_from_stack(conn: &Connection, photo: i64) -> Result<()> {
    let Some(stack) = stack_of(conn, photo)? else {
        return Ok(());
    };
    conn.execute(
        "UPDATE photos SET stack_id = NULL, stack_hidden = 0, stack_manual = 1 WHERE id = ?1",
        [photo],
    )?;
    refresh(conn, stack)
}

/// Pone al día una pila después de cambiar sus fotos: deja de existir si le
/// queda menos de 2, elige portada si la suya ya no está y recalcula
/// `stack_hidden`.
pub(crate) fn refresh(conn: &Connection, stack: i64) -> Result<()> {
    let by_date = members_by_date(conn, stack)?;
    if by_date.len() < 2 {
        conn.execute(
            "UPDATE photos SET stack_id = NULL, stack_hidden = 0 WHERE stack_id = ?1",
            [stack],
        )?;
        conn.execute("DELETE FROM stacks WHERE id = ?1", [stack])?;
        return Ok(());
    }
    let cover: i64 = conn.query_row("SELECT cover_id FROM stacks WHERE id = ?1", [stack], |r| {
        r.get(0)
    })?;
    let cover = if by_date.contains(&cover) {
        cover
    } else {
        conn.execute(
            "UPDATE stacks SET cover_id = ?2 WHERE id = ?1",
            params![stack, by_date[0]],
        )?;
        by_date[0]
    };
    conn.execute(
        "UPDATE photos SET stack_hidden = (id <> ?2) WHERE stack_id = ?1",
        params![stack, cover],
    )?;
    Ok(())
}

fn photo_exists(conn: &Connection, id: i64) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM photos WHERE id = ?1)",
        [id],
        |r| r.get(0),
    )?)
}

fn stack_exists(conn: &Connection, stack: i64) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM stacks WHERE id = ?1)",
        [stack],
        |r| r.get(0),
    )?)
}

/// Pila de una foto. Error si la foto no existe.
fn stack_of(conn: &Connection, photo: i64) -> Result<Option<i64>> {
    conn.query_row("SELECT stack_id FROM photos WHERE id = ?1", [photo], |r| {
        r.get(0)
    })
    .optional()?
    .ok_or(Error::PhotoNotFound(photo))
}

/// Fotos de una pila por fecha, de la más antigua a la más reciente.
fn members_by_date(conn: &Connection, stack: i64) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT id FROM photos WHERE stack_id = ?1 ORDER BY {DATE}, id"
    ))?;
    let ids = stmt
        .query_map([stack], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<i64>>>()?;
    Ok(ids)
}

/// Fotos de una pila: la portada primero y luego por fecha.
fn members_of(conn: &Connection, stack: i64) -> Result<Vec<i64>> {
    let cover: Option<i64> = conn
        .query_row("SELECT cover_id FROM stacks WHERE id = ?1", [stack], |r| {
            r.get(0)
        })
        .optional()?;
    let mut ids = members_by_date(conn, stack)?;
    if let Some(pos) = cover.and_then(|c| ids.iter().position(|&id| id == c)) {
        let cover = ids.remove(pos);
        ids.insert(0, cover);
    }
    Ok(ids)
}

fn expand(conn: &Connection, ids: &[i64]) -> Result<Vec<i64>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for &id in ids {
        let stack: Option<Option<i64>> = conn
            .query_row("SELECT stack_id FROM photos WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        match stack {
            Some(Some(stack)) => {
                for member in members_of(conn, stack)? {
                    if seen.insert(member) {
                        out.push(member);
                    }
                }
            }
            Some(None) if seen.insert(id) => out.push(id),
            // Repetida, o una foto que ya no existe: no arrastra nada.
            Some(None) | None => {}
        }
    }
    Ok(out)
}

/// Todas archivadas o todas sin archivar, y ninguna en la Papelera.
fn check_same_place(conn: &Connection, ids: &[i64]) -> Result<()> {
    let mut stmt = conn.prepare_cached(
        "SELECT archived_at IS NOT NULL, trashed_at IS NOT NULL FROM photos WHERE id = ?1",
    )?;
    let mut archived = None;
    for &id in ids {
        let (a, t): (bool, bool) = stmt.query_row([id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        if t || archived.is_some_and(|prev| prev != a) {
            return Err(Error::StackMixedPlaces);
        }
        archived = Some(a);
    }
    Ok(())
}

/// Portada de una pila nueva hecha con `members`: la portada de la pila que
/// sale antes en la cuadrícula (la más reciente) o, si ninguna estaba
/// apilada, la foto más reciente.
fn first_cover(conn: &Connection, members: &[i64]) -> Result<i64> {
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT {DATE}, id, EXISTS (SELECT 1 FROM stacks s WHERE s.cover_id = photos.id
                                    AND s.id = photos.stack_id)
         FROM photos WHERE id = ?1"
    ))?;
    let mut best_cover: Option<(i64, i64)> = None;
    let mut best: Option<(i64, i64)> = None;
    for &id in members {
        let (date, id, is_cover): (i64, i64, bool) =
            stmt.query_row([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        if is_cover && best_cover.is_none_or(|b| (date, id) > b) {
            best_cover = Some((date, id));
        }
        if best.is_none_or(|b| (date, id) > b) {
            best = Some((date, id));
        }
    }
    let (_, id) = best_cover.or(best).ok_or(Error::StackTooSmall)?;
    Ok(id)
}

/// Cómo están ahora las fotos `ids` y las pilas en las que están.
fn snapshot(conn: &Connection, ids: &[i64]) -> Result<StackUndo> {
    let mut photos = Vec::new();
    let mut stacks = Vec::new();
    let mut seen = HashSet::new();
    let mut photo =
        conn.prepare_cached("SELECT stack_id, stack_manual FROM photos WHERE id = ?1")?;
    let mut stack =
        conn.prepare_cached("SELECT cover_id, kind, created_at FROM stacks WHERE id = ?1")?;
    for &id in ids {
        let (stack_id, stack_manual): (Option<i64>, bool) =
            photo.query_row([id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        photos.push(SavedPhoto {
            id,
            stack_id,
            stack_manual,
        });
        if let Some(s) = stack_id.filter(|s| seen.insert(*s)) {
            stacks.push(stack.query_row([s], |r| {
                Ok(SavedStack {
                    id: s,
                    cover_id: r.get(0)?,
                    kind: StackKind::from_sql(&r.get::<_, String>(1)?)?,
                    created_at: r.get(2)?,
                })
            })?);
        }
    }
    Ok(StackUndo {
        stacks,
        created: Vec::new(),
        photos,
    })
}

/// Lo que incumple las invariantes de las pilas, para las pruebas. Vacío si
/// todo está bien.
#[cfg(test)]
pub(crate) fn invariant_violations(conn: &Connection) -> Vec<String> {
    let query = |sql: &str| -> Vec<String> {
        let mut stmt = conn.prepare(sql).unwrap();
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    };
    let mut problems = Vec::new();
    problems.extend(query(
        "SELECT 'la pila ' || s.id || ' tiene ' || count(p.id) || ' fotos'
         FROM stacks s LEFT JOIN photos p ON p.stack_id = s.id
         GROUP BY s.id HAVING count(p.id) < 2",
    ));
    problems.extend(query(
        "SELECT 'la portada de la pila ' || s.id || ' no está en ella'
         FROM stacks s
         WHERE NOT EXISTS (SELECT 1 FROM photos p WHERE p.id = s.cover_id AND p.stack_id = s.id)",
    ));
    problems.extend(query(
        "SELECT 'stack_hidden incorrecto en la foto ' || p.id
         FROM photos p LEFT JOIN stacks s ON s.id = p.stack_id
         WHERE p.stack_hidden <> (p.stack_id IS NOT NULL AND p.id <> s.cover_id)",
    ));
    problems.extend(query(
        "SELECT 'la pila ' || stack_id || ' mezcla sitios'
         FROM photos WHERE stack_id IS NOT NULL
         GROUP BY stack_id
         HAVING count(DISTINCT archived_at IS NULL) > 1 OR max(trashed_at IS NOT NULL) = 1
                AND min(trashed_at IS NOT NULL) = 0",
    ));
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: i64 = 1_791_892_800; // 2026-10-13 12:00 UTC

    /// Biblioteca con `n` fotos sueltas, de la más antigua (id 1) a la más
    /// reciente, separadas un minuto.
    fn library(n: i64) -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        for i in 1..=n {
            lib.conn
                .execute(
                    "INSERT INTO photos (rel_path, size_bytes, modified_at, taken_at, imported_at)
                     VALUES (?1, 1, ?2, ?2, 0)",
                    params![format!("Originales/2026/10/IMG_{i:04}.jpg"), BASE + i * 60],
                )
                .unwrap();
        }
        (dir, lib)
    }

    /// Estado de las pilas de todas las fotos, para comparar antes y después.
    /// Por foto: id, pila, escondida y manual. Por pila: id, portada y tipo.
    type Estado = (Vec<(i64, Option<i64>, bool, bool)>, Vec<(i64, i64, String)>);

    fn estado(lib: &Library) -> Estado {
        let mut stmt = lib
            .conn
            .prepare("SELECT id, stack_id, stack_hidden, stack_manual FROM photos ORDER BY id")
            .unwrap();
        let fotos = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let mut stmt = lib
            .conn
            .prepare("SELECT id, cover_id, kind FROM stacks ORDER BY id")
            .unwrap();
        let pilas = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        (fotos, pilas)
    }

    fn sin_problemas(lib: &Library) {
        let problemas = invariant_violations(&lib.conn);
        assert!(problemas.is_empty(), "{problemas:?}");
    }

    fn pila_de(lib: &Library, foto: i64) -> Option<i64> {
        stack_of(&lib.conn, foto).unwrap()
    }

    #[test]
    fn apilar_esconde_todas_menos_la_portada() {
        let (_dir, mut lib) = library(4);
        lib.stack_photos(&[1, 2, 3]).unwrap();
        let pila = pila_de(&lib, 1).unwrap();
        // Sin pilas entre ellas, la portada es la más reciente.
        assert_eq!(lib.stack_members(pila).unwrap(), vec![3, 1, 2]);
        let ocultas: Vec<i64> = lib
            .conn
            .prepare("SELECT id FROM photos WHERE stack_hidden = 1 ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(ocultas, vec![1, 2]);
        assert_eq!(pila_de(&lib, 4), None);
        let kind: String = lib
            .conn
            .query_row("SELECT kind FROM stacks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(kind, "manual");
        sin_problemas(&lib);
    }

    #[test]
    fn apilar_una_pila_con_sueltas_conserva_su_portada() {
        let (_dir, mut lib) = library(5);
        lib.stack_photos(&[1, 2]).unwrap();
        lib.set_stack_cover(1).unwrap();
        // La tarjeta de la pila es la foto 1; se apila con las 4 y 5.
        lib.stack_photos(&[4, 1, 5]).unwrap();
        let nueva = pila_de(&lib, 5).unwrap();
        // La pila vieja desaparece: solo queda una (SQLite puede reutilizar su id).
        assert_eq!(estado(&lib).1.len(), 1);
        assert_eq!(lib.stack_members(nueva).unwrap(), vec![1, 2, 4, 5]);
        sin_problemas(&lib);
    }

    #[test]
    fn apilar_necesita_dos_fotos_del_mismo_sitio() {
        let (_dir, mut lib) = library(4);
        assert!(matches!(lib.stack_photos(&[1]), Err(Error::StackTooSmall)));
        assert!(matches!(
            lib.stack_photos(&[1, 1]),
            Err(Error::StackTooSmall)
        ));
        assert!(matches!(
            lib.stack_photos(&[1, 99]),
            Err(Error::PhotoNotFound(99))
        ));
        lib.conn
            .execute("UPDATE photos SET archived_at = 1 WHERE id = 2", [])
            .unwrap();
        lib.conn
            .execute("UPDATE photos SET trashed_at = 1 WHERE id = 4", [])
            .unwrap();
        assert!(matches!(
            lib.stack_photos(&[1, 2]),
            Err(Error::StackMixedPlaces)
        ));
        assert!(matches!(
            lib.stack_photos(&[1, 4]),
            Err(Error::StackMixedPlaces)
        ));
        // Dos archivadas sí.
        lib.conn
            .execute("UPDATE photos SET archived_at = 1 WHERE id = 3", [])
            .unwrap();
        lib.stack_photos(&[2, 3]).unwrap();
        assert_eq!(estado(&lib).1.len(), 1);
        sin_problemas(&lib);
    }

    #[test]
    fn desapilar_deja_las_fotos_sueltas_y_fuera_de_la_agrupacion() {
        let (_dir, mut lib) = library(3);
        lib.stack_photos(&[1, 2, 3]).unwrap();
        let pila = pila_de(&lib, 1).unwrap();
        lib.unstack(&[pila]).unwrap();
        let (fotos, pilas) = estado(&lib);
        assert!(pilas.is_empty());
        assert!(fotos.iter().all(|f| f.1.is_none() && !f.2 && f.3));
        assert!(matches!(lib.unstack(&[pila]), Err(Error::StackNotFound(_))));
        sin_problemas(&lib);
    }

    #[test]
    fn cambiar_la_portada() {
        let (_dir, mut lib) = library(3);
        lib.stack_photos(&[1, 2, 3]).unwrap();
        let pila = pila_de(&lib, 1).unwrap();
        lib.set_stack_cover(2).unwrap();
        assert_eq!(lib.stack_members(pila).unwrap(), vec![2, 1, 3]);
        sin_problemas(&lib);
        let (_dir2, mut otra) = library(1);
        assert!(matches!(otra.set_stack_cover(1), Err(Error::NotInStack(1))));
        assert!(matches!(
            otra.set_stack_cover(7),
            Err(Error::PhotoNotFound(7))
        ));
    }

    #[test]
    fn deshacer_deja_todo_como_estaba() {
        let (_dir, mut lib) = library(6);
        lib.stack_photos(&[1, 2]).unwrap();
        lib.stack_photos(&[3, 4]).unwrap();
        let antes = estado(&lib);

        let undo = lib.stack_photos(&[2, 4, 5]).unwrap();
        lib.restore_stacks(&undo).unwrap();
        assert_eq!(estado(&lib), antes);
        sin_problemas(&lib);

        let pila = pila_de(&lib, 3).unwrap();
        let undo = lib.unstack(&[pila]).unwrap();
        lib.restore_stacks(&undo).unwrap();
        assert_eq!(estado(&lib), antes);

        let undo = lib.set_stack_cover(1).unwrap();
        lib.restore_stacks(&undo).unwrap();
        assert_eq!(estado(&lib), antes);
        sin_problemas(&lib);

        // Lo que se guarda para deshacer pasa por la interfaz como JSON.
        let undo = lib.stack_photos(&[5, 6]).unwrap();
        let json = serde_json::to_string(&undo).unwrap();
        let vuelta: StackUndo = serde_json::from_str(&json).unwrap();
        lib.restore_stacks(&vuelta).unwrap();
        assert_eq!(estado(&lib), antes);
    }

    #[test]
    fn sacar_la_portada_elige_la_siguiente_y_una_sola_foto_no_es_pila() {
        let (_dir, mut lib) = library(3);
        lib.stack_photos(&[1, 2, 3]).unwrap();
        let pila = pila_de(&lib, 1).unwrap();
        remove_from_stack(&lib.conn, 3).unwrap();
        // La siguiente por fecha: la más antigua que queda.
        assert_eq!(lib.stack_members(pila).unwrap(), vec![1, 2]);
        sin_problemas(&lib);
        remove_from_stack(&lib.conn, 1).unwrap();
        assert!(!stack_exists(&lib.conn, pila).unwrap());
        let (fotos, _) = estado(&lib);
        assert!(fotos.iter().all(|f| f.1.is_none() && !f.2));
        // Apiladas a mano y luego sacadas: todas quedan fuera de la
        // agrupación automática.
        assert_eq!(
            fotos.iter().map(|f| f.3).collect::<Vec<_>>(),
            vec![true, true, true]
        );
        sin_problemas(&lib);
    }

    #[test]
    fn expandir_las_tarjetas_a_sus_pilas() {
        let (_dir, mut lib) = library(5);
        lib.stack_photos(&[2, 3, 4]).unwrap();
        lib.set_stack_cover(3).unwrap();
        assert_eq!(
            lib.expand_stacks(&[5, 3, 1, 4, 99]).unwrap(),
            vec![5, 3, 2, 4, 1]
        );
    }

    #[test]
    fn borrar_una_pila_suelta_sus_fotos() {
        // La clave ajena deja `stack_id` en NULL si alguien borra la pila.
        let (_dir, mut lib) = library(2);
        lib.stack_photos(&[1, 2]).unwrap();
        lib.conn.execute("DELETE FROM stacks", []).unwrap();
        assert_eq!(pila_de(&lib, 1), None);
    }
}
