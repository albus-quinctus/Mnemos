//! Agrupación automática de pilas (spec 023): reglas 1 (RAW + JPEG) y 2
//! (editada de Google Fotos).
//!
//! Cada foto se examina una sola vez (`stack_auto_at`). Las reglas buscan
//! pareja también entre las fotos ya examinadas de la misma carpeta, pero un
//! grupo solo se apila si alguna de sus fotos está pendiente. Así, lo que el
//! usuario desapila no vuelve a apilarse solo. Solo se mira la base de datos:
//! los originales no se tocan.

use std::collections::{BTreeMap, HashMap};
use std::time::SystemTime;

use rusqlite::{Connection, params};

use crate::error::Result;
use crate::files::unix_secs;
use crate::library::Library;
use crate::stacks::{StackKind, refresh};

/// Sufijos que Google Fotos añade a las copias editadas (los de `takeout.rs`).
const EDITED_SUFFIXES: &[&str] = &["-editada", "-editado", "-edited"];

/// Diferencia máxima, sin llegar, entre las fechas de un RAW y su JPEG.
const RAW_JPEG_MAX_SECS: i64 = 2;

/// Lo que las reglas necesitan saber de cada foto.
#[derive(Debug)]
struct Row {
    id: i64,
    /// Nombre sin extensión, en minúsculas.
    stem: String,
    raw: bool,
    taken: Option<i64>,
    archived: bool,
    pending: bool,
    stack: Option<i64>,
    /// Su pila es de tipo RAW + JPEG.
    in_raw_jpeg: bool,
}

impl Library {
    /// Aplica la agrupación automática a las fotos que aún no se han
    /// examinado y las marca como examinadas, todo en una transacción.
    /// Devuelve cuántas pilas se han creado o ampliado.
    pub fn auto_stack_pending(&mut self) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let changed = auto_stack(&tx)?;
        tx.commit()?;
        Ok(changed)
    }
}

fn auto_stack(conn: &Connection) -> Result<usize> {
    let pending: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM photos WHERE stack_auto_at IS NULL)",
        [],
        |r| r.get(0),
    )?;
    if !pending {
        return Ok(0);
    }
    let now = unix_secs(SystemTime::now());
    let mut changed = 0;
    for rows in folders(conn)?.values_mut() {
        if rows.iter().any(|r| r.pending) {
            changed += raw_jpeg(conn, rows, now)?;
            changed += edited(conn, rows, now)?;
        }
    }
    conn.execute(
        "UPDATE photos SET stack_auto_at = ?1 WHERE stack_auto_at IS NULL",
        [now],
    )?;
    Ok(changed)
}

/// Las fotos que pueden entrar en una pila automática, por carpeta. Quedan
/// fuera los vídeos, la Papelera y las fotos que el usuario apiló o desapiló.
fn folders(conn: &Connection) -> Result<BTreeMap<String, Vec<Row>>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.rel_path, p.taken_at, p.archived_at IS NOT NULL,
                p.stack_auto_at IS NULL, p.stack_id, s.kind
         FROM photos p LEFT JOIN stacks s ON s.id = p.stack_id
         WHERE p.media_kind = 'foto' AND p.trashed_at IS NULL AND p.stack_manual = 0
         ORDER BY p.rel_path",
    )?;
    let mut rows = stmt.query([])?;
    let mut out: BTreeMap<String, Vec<Row>> = BTreeMap::new();
    while let Some(r) = rows.next()? {
        let rel: String = r.get(1)?;
        let (dir, name) = rel.rsplit_once('/').unwrap_or(("", &rel));
        let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
        let ext = ext.to_lowercase();
        let row = Row {
            id: r.get(0)?,
            stem: stem.to_lowercase(),
            raw: mnemos_imaging::RAW_EXTENSIONS.contains(&ext.as_str()),
            taken: r.get(2)?,
            archived: r.get(3)?,
            pending: r.get(4)?,
            stack: r.get(5)?,
            in_raw_jpeg: r.get::<_, Option<String>>(6)?.as_deref()
                == Some(StackKind::RawJpeg.as_sql()),
        };
        out.entry(dir.to_owned()).or_default().push(row);
    }
    Ok(out)
}

/// Regla 1: un RAW y una foto que no es RAW con el mismo nombre y, si las dos
/// tienen fecha, con menos de 2 s de diferencia. Portada: la que no es RAW.
/// Si hay más de un RAW o más de una foto con ese nombre, no se apila nada:
/// mejor no apilar que apilar mal.
fn raw_jpeg(conn: &Connection, rows: &mut [Row], now: i64) -> Result<usize> {
    let mut groups: HashMap<&str, (Vec<usize>, Vec<usize>)> = HashMap::new();
    for (i, row) in rows.iter().enumerate() {
        let group = groups.entry(row.stem.as_str()).or_default();
        if row.raw {
            group.0.push(i);
        } else {
            group.1.push(i);
        }
    }
    let mut pairs: Vec<(usize, usize)> = groups
        .into_values()
        .filter_map(|(raws, others)| match (&raws[..], &others[..]) {
            ([raw], [other]) => Some((*raw, *other)),
            _ => None,
        })
        .collect();
    pairs.sort_unstable();

    let mut changed = 0;
    for (raw, other) in pairs {
        let (a, b) = (&rows[raw], &rows[other]);
        let close = match (a.taken, b.taken) {
            (Some(x), Some(y)) => (x - y).abs() < RAW_JPEG_MAX_SECS,
            _ => true,
        };
        if a.stack.is_none()
            && b.stack.is_none()
            && (a.pending || b.pending)
            && a.archived == b.archived
            && close
        {
            let stack = create(conn, StackKind::RawJpeg, b.id, &[a.id, b.id], now)?;
            for i in [raw, other] {
                rows[i].stack = Some(stack);
                rows[i].in_raw_jpeg = true;
            }
            changed += 1;
        }
    }
    Ok(changed)
}

/// Regla 2: una foto `…-editada` (o `-editado`, `-edited`) y su original con
/// el mismo nombre sin el sufijo. Portada: la editada. Si el original ya está
/// en una pila RAW + JPEG, la editada entra en ella y pasa a ser su portada.
fn edited(conn: &Connection, rows: &mut [Row], now: i64) -> Result<usize> {
    let mut changed = 0;
    for e in 0..rows.len() {
        if rows[e].stack.is_some() {
            continue;
        }
        let Some(base) = EDITED_SUFFIXES
            .iter()
            .find_map(|s| rows[e].stem.strip_suffix(s))
        else {
            continue;
        };
        let originals: Vec<usize> = (0..rows.len())
            .filter(|&i| i != e && rows[i].stem == base)
            .collect();
        let Some(&first) = originals.first() else {
            continue;
        };
        let edit = &rows[e];
        let group_pending = edit.pending || originals.iter().any(|&i| rows[i].pending);
        let same_place = originals.iter().all(|&i| rows[i].archived == edit.archived);
        if !group_pending || !same_place {
            continue;
        }
        let stacks: Vec<Option<i64>> = originals.iter().map(|&i| rows[i].stack).collect();
        let target = match (stacks[0], originals.len()) {
            // Un original suelto: pila nueva.
            (None, 1) => None,
            // Todos los originales en la misma pila RAW + JPEG: se une a ella.
            (Some(s), _) if stacks.iter().all(|x| *x == Some(s)) && rows[first].in_raw_jpeg => {
                Some(s)
            }
            _ => continue,
        };
        let edit_id = edit.id;
        let stack = match target {
            None => {
                let members = [rows[first].id, edit_id];
                let stack = create(conn, StackKind::Edited, edit_id, &members, now)?;
                rows[first].stack = Some(stack);
                stack
            }
            Some(stack) => {
                conn.execute(
                    "UPDATE photos SET stack_id = ?1 WHERE id = ?2",
                    params![stack, edit_id],
                )?;
                conn.execute(
                    "UPDATE stacks SET cover_id = ?2 WHERE id = ?1",
                    params![stack, edit_id],
                )?;
                refresh(conn, stack)?;
                rows[e].in_raw_jpeg = true;
                stack
            }
        };
        rows[e].stack = Some(stack);
        changed += 1;
    }
    Ok(changed)
}

/// Crea una pila automática con esas fotos y esa portada.
fn create(
    conn: &Connection,
    kind: StackKind,
    cover: i64,
    members: &[i64],
    now: i64,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO stacks (cover_id, kind, created_at) VALUES (?1, ?2, ?3)",
        params![cover, kind.as_sql(), now],
    )?;
    let stack = conn.last_insert_rowid();
    for id in members {
        conn.execute(
            "UPDATE photos SET stack_id = ?1 WHERE id = ?2",
            params![stack, id],
        )?;
    }
    refresh(conn, stack)?;
    Ok(stack)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stacks::invariant_violations;

    const BASE: i64 = 1_791_892_800; // 2026-10-13 12:00 UTC

    struct Foto<'a> {
        path: &'a str,
        taken: Option<i64>,
    }

    fn foto(path: &str) -> Foto<'_> {
        Foto {
            path,
            taken: Some(BASE),
        }
    }

    fn library(fotos: &[Foto]) -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        for f in fotos {
            add(&lib, f);
        }
        (dir, lib)
    }

    fn add(lib: &Library, f: &Foto) -> i64 {
        lib.conn
            .execute(
                "INSERT INTO photos (rel_path, size_bytes, modified_at, taken_at, imported_at)
                 VALUES (?1, 1, ?2, ?3, 0)",
                params![format!("Originales/2026/10/{}", f.path), BASE, f.taken],
            )
            .unwrap();
        lib.conn.last_insert_rowid()
    }

    /// Pilas como (tipo, portada, fotos por id), ordenadas.
    fn pilas(lib: &Library) -> Vec<(String, i64, Vec<i64>)> {
        let mut stmt = lib
            .conn
            .prepare("SELECT id, kind, cover_id FROM stacks ORDER BY cover_id")
            .unwrap();
        let rows: Vec<(i64, String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        rows.into_iter()
            .map(|(id, kind, cover)| {
                let mut stmt = lib
                    .conn
                    .prepare("SELECT id FROM photos WHERE stack_id = ?1 ORDER BY id")
                    .unwrap();
                let ids = stmt
                    .query_map([id], |r| r.get(0))
                    .unwrap()
                    .collect::<rusqlite::Result<_>>()
                    .unwrap();
                (kind, cover, ids)
            })
            .collect()
    }

    fn agrupar(lib: &mut Library) -> usize {
        let n = lib.auto_stack_pending().unwrap();
        let problemas = invariant_violations(&lib.conn);
        assert!(problemas.is_empty(), "{problemas:?}");
        n
    }

    fn pendientes(lib: &Library) -> i64 {
        lib.conn
            .query_row(
                "SELECT count(*) FROM photos WHERE stack_auto_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap()
    }

    #[test]
    fn raw_y_jpeg_con_el_mismo_nombre_se_apilan_con_el_jpeg_de_portada() {
        let (_dir, mut lib) = library(&[
            foto("IMG_0001.CR2"),
            foto("img_0001.JPG"),
            foto("IMG_0002.jpg"),
        ]);
        assert_eq!(agrupar(&mut lib), 1);
        assert_eq!(pilas(&lib), vec![("raw_jpeg".into(), 2, vec![1, 2])]);
        assert_eq!(pendientes(&lib), 0);
    }

    #[test]
    fn raw_y_jpeg_con_fechas_lejanas_no_se_apilan() {
        let (_dir, mut lib) = library(&[
            foto("IMG_0001.CR2"),
            Foto {
                path: "IMG_0001.JPG",
                taken: Some(BASE + 2),
            },
            // Sin fecha en una de las dos: basta el nombre.
            Foto {
                path: "DSC_0007.NEF",
                taken: None,
            },
            foto("DSC_0007.jpg"),
        ]);
        assert_eq!(agrupar(&mut lib), 1);
        assert_eq!(pilas(&lib), vec![("raw_jpeg".into(), 4, vec![3, 4])]);
    }

    #[test]
    fn nombres_repetidos_y_otras_carpetas_no_se_emparejan() {
        let (_dir, mut lib) = library(&[
            foto("IMG_0001.CR2"),
            foto("IMG_0001 (1).JPG"),
            foto("otra/IMG_0001.JPG"),
            // Dos JPEG para un RAW: ambiguo, no se apila.
            foto("IMG_0002.dng"),
            foto("IMG_0002.jpg"),
            foto("IMG_0002.heic"),
        ]);
        assert_eq!(agrupar(&mut lib), 0);
        assert!(pilas(&lib).is_empty());
    }

    #[test]
    fn la_editada_se_apila_con_su_original_y_es_la_portada() {
        let (_dir, mut lib) = library(&[
            foto("IMG-20181215-WA0003.jpg"),
            foto("IMG-20181215-WA0003-editada.jpg"),
            foto("PXL_1.jpg"),
            foto("PXL_1-EDITED.jpg"),
            foto("Sin-original-editado.jpg"),
        ]);
        assert_eq!(agrupar(&mut lib), 2);
        assert_eq!(
            pilas(&lib),
            vec![
                ("editada".into(), 2, vec![1, 2]),
                ("editada".into(), 4, vec![3, 4]),
            ]
        );
    }

    #[test]
    fn la_editada_entra_en_la_pila_raw_jpeg_y_pasa_a_portada() {
        let (_dir, mut lib) = library(&[
            foto("IMG_0001.CR2"),
            foto("IMG_0001.JPG"),
            foto("IMG_0001-editada.jpg"),
        ]);
        agrupar(&mut lib);
        assert_eq!(pilas(&lib), vec![("raw_jpeg".into(), 3, vec![1, 2, 3])]);
    }

    #[test]
    fn una_foto_ya_examinada_se_empareja_con_una_nueva() {
        let (_dir, mut lib) = library(&[foto("IMG_0001.CR2")]);
        assert_eq!(agrupar(&mut lib), 0);
        let jpeg = add(&lib, &foto("IMG_0001.JPG"));
        assert_eq!(agrupar(&mut lib), 1);
        assert_eq!(pilas(&lib), vec![("raw_jpeg".into(), jpeg, vec![1, jpeg])]);
    }

    #[test]
    fn lo_desapilado_no_se_vuelve_a_apilar() {
        let (_dir, mut lib) = library(&[foto("IMG_0001.CR2"), foto("IMG_0001.JPG")]);
        agrupar(&mut lib);
        let pila: i64 = lib
            .conn
            .query_row("SELECT id FROM stacks", [], |r| r.get(0))
            .unwrap();
        lib.unstack(&[pila]).unwrap();
        // Una foto nueva con el mismo nombre tampoco las recoge.
        add(&lib, &foto("IMG_0001-edited.jpg"));
        assert_eq!(agrupar(&mut lib), 0);
        assert!(pilas(&lib).is_empty());
    }

    #[test]
    fn examinadas_sin_pareja_no_se_apilan_al_volver_a_mirar() {
        let (_dir, mut lib) = library(&[foto("IMG_0001.CR2"), foto("IMG_0001.JPG")]);
        lib.conn
            .execute("UPDATE photos SET stack_auto_at = 1", [])
            .unwrap();
        assert_eq!(agrupar(&mut lib), 0);
    }

    #[test]
    fn nunca_mezcla_archivadas_papelera_ni_videos() {
        let (_dir, mut lib) = library(&[
            foto("A.CR2"),
            foto("A.JPG"),
            foto("B.CR2"),
            foto("B.JPG"),
            foto("C.jpg"),
            foto("C-editada.jpg"),
        ]);
        lib.conn
            .execute_batch(
                "UPDATE photos SET archived_at = 1 WHERE id = 1;
                 UPDATE photos SET trashed_at = 1 WHERE id = 3;
                 UPDATE photos SET media_kind = 'video' WHERE id = 5;",
            )
            .unwrap();
        assert_eq!(agrupar(&mut lib), 0);
        assert_eq!(pendientes(&lib), 0);
    }

    #[test]
    fn indexar_agrupa() {
        let (_dir, mut lib) = library(&[foto("IMG_0001.CR2"), foto("IMG_0001.JPG")]);
        lib.index().unwrap();
        assert_eq!(pilas(&lib).len(), 1);
    }
}
