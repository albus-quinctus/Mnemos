//! Fotos repetidas byte a byte (spec 010).
//!
//! Dos fotos son duplicados exactos si tienen la misma huella BLAKE3
//! (`content_hash`). Este módulo solo consulta: no borra, no mueve y no marca
//! nada, ni en el disco ni en la base de datos. Decidir qué hacer con cada
//! copia es cosa del usuario.

use std::io;

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::library::{Library, ORIGINALS_DIR};
use crate::paths::RelPath;
use crate::timeline::DATE;

/// Grupos por página como máximo, pida lo que pida la interfaz.
pub const MAX_GROUPS_PAGE: u32 = 200;

/// Cuántas fotos repetidas hay en la biblioteca.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct DuplicateSummary {
    /// Huellas con dos o más fotos.
    pub groups: u64,
    /// En cada grupo, todas las copias menos una.
    pub extra_copies: u64,
    /// Lo que ocupan esas copias de más.
    pub extra_bytes: u64,
}

/// Una de las copias de un grupo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateCopy {
    pub id: i64,
    pub rel_path: String,
    pub size_bytes: u64,
    /// La fecha de la cuadrícula: captura o, si falta, modificación.
    pub date: i64,
    pub favorite: bool,
    pub archived: bool,
    /// `false` si el archivo ya no está en el disco (foto desaparecida).
    pub exists: bool,
}

/// Fotos con la misma huella, por orden de llegada a la biblioteca. Ese orden
/// no es una recomendación de cuál conservar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicateGroup {
    pub content_hash: String,
    /// La fecha más reciente de las copias: ordena los grupos.
    pub date: i64,
    pub copies: Vec<DuplicateCopy>,
}

/// Dónde sigue la página siguiente: justo después de este grupo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateCursor {
    pub date: i64,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DuplicatePage {
    pub groups: Vec<DuplicateGroup>,
    /// `None` cuando ya no quedan grupos.
    pub next: Option<DuplicateCursor>,
}

impl Library {
    /// Resumen de las fotos repetidas. Las fotos sin huella no cuentan.
    pub fn duplicate_summary(&self) -> Result<DuplicateSummary> {
        Ok(self.conn.query_row(
            "SELECT count(*), coalesce(sum(n - 1), 0), coalesce(sum((n - 1) * size), 0)
             FROM (SELECT count(*) AS n, min(size_bytes) AS size
                   FROM photos WHERE content_hash IS NOT NULL
                   GROUP BY content_hash HAVING n > 1)",
            [],
            |r| {
                Ok(DuplicateSummary {
                    groups: r.get(0)?,
                    extra_copies: r.get(1)?,
                    extra_bytes: r.get(2)?,
                })
            },
        )?)
    }

    /// Hasta `limit` grupos a partir de `after` (o desde el más reciente), con
    /// todas sus copias. Comprueba en el disco si cada copia sigue ahí.
    pub fn duplicate_groups(
        &self,
        after: Option<&DuplicateCursor>,
        limit: u32,
    ) -> Result<DuplicatePage> {
        if !self.root.join(ORIGINALS_DIR).is_dir() {
            return Err(Error::Io(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "no se encuentra {} en {}: ¿está conectado el disco?",
                    ORIGINALS_DIR,
                    self.root.display()
                ),
            )));
        }
        let limit = limit.clamp(1, MAX_GROUPS_PAGE);
        let (date, hash) = after.map_or((i64::MAX, ""), |c| (c.date, c.content_hash.as_str()));
        // Sin cursor, cualquier huella vale: `i64::MAX` ya deja pasar todas las
        // fechas y la comparación de huellas solo decide en caso de empate.
        let hash_bound = if after.is_some() { hash } else { "\u{10FFFF}" };
        let mut stmt = self.conn.prepare(&format!(
            "SELECT content_hash, max({DATE}) AS d
             FROM photos WHERE content_hash IS NOT NULL
             GROUP BY content_hash
             HAVING count(*) > 1 AND (d, content_hash) < (?1, ?2)
             ORDER BY d DESC, content_hash DESC
             LIMIT ?3"
        ))?;
        // Se pide uno de más para saber si hay página siguiente.
        let mut heads = stmt
            .query_map(params![date, hash_bound, i64::from(limit) + 1], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let more = heads.len() > limit as usize;
        heads.truncate(limit as usize);

        let mut copies_stmt = self.conn.prepare(&format!(
            "SELECT id, rel_path, size_bytes, {DATE}, favorite, archived_at IS NOT NULL
             FROM photos WHERE content_hash = ?1
             ORDER BY imported_at, id"
        ))?;
        let mut groups = Vec::with_capacity(heads.len());
        for (content_hash, date) in heads {
            let copies = copies_stmt
                .query_map([&content_hash], |r| {
                    Ok(DuplicateCopy {
                        id: r.get(0)?,
                        rel_path: r.get(1)?,
                        size_bytes: r.get(2)?,
                        date: r.get(3)?,
                        favorite: r.get(4)?,
                        archived: r.get(5)?,
                        exists: false,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .map(|mut c| {
                    c.exists = RelPath::new(&c.rel_path)
                        .map(|p| p.to_path(&self.root).is_file())
                        .unwrap_or(false);
                    c
                })
                .collect();
            groups.push(DuplicateGroup {
                content_hash,
                date,
                copies,
            });
        }
        let next = if more {
            groups.last().map(|g| DuplicateCursor {
                date: g.date,
                content_hash: g.content_hash.clone(),
            })
        } else {
            None
        };
        Ok(DuplicatePage { groups, next })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::index::tests::{set_mtime, write_png};

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        (dir, lib)
    }

    fn originals(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path().join("Originales")
    }

    /// Todas las rutas de los grupos, recorriendo las páginas de `limit` en
    /// `limit`.
    fn all_groups(lib: &Library, limit: u32) -> Vec<DuplicateGroup> {
        let mut out = Vec::new();
        let mut cursor = None;
        loop {
            let page = lib.duplicate_groups(cursor.as_ref(), limit).unwrap();
            out.extend(page.groups);
            match page.next {
                Some(c) => cursor = Some(c),
                None => return out,
            }
        }
    }

    fn paths(group: &DuplicateGroup) -> Vec<&str> {
        group.copies.iter().map(|c| c.rel_path.as_str()).collect()
    }

    /// Contenido de todos los archivos de `dir`, para ver que nada cambia.
    fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        let mut pending = vec![dir.to_owned()];
        while let Some(d) = pending.pop() {
            for e in fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    pending.push(p);
                } else {
                    out.push((p.display().to_string(), fs::read(&p).unwrap()));
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn agrupa_archivos_identicos_y_deja_fuera_los_distintos() {
        let (dir, mut lib) = library();
        let o = originals(&dir);
        write_png(&o.join("2024/05/playa.png"), 10, 10, b"a");
        write_png(&o.join("copias/playa (1).PNG"), 10, 10, b"a");
        write_png(&o.join("2024/05/otra.png"), 10, 10, b"b");
        lib.index().unwrap();

        let groups = all_groups(&lib, 50);
        assert_eq!(groups.len(), 1);
        let mut p = paths(&groups[0]);
        p.sort();
        assert_eq!(
            p,
            [
                "Originales/2024/05/playa.png",
                "Originales/copias/playa (1).PNG"
            ]
        );
        assert!(groups[0].copies.iter().all(|c| c.exists));
    }

    #[test]
    fn el_resumen_cuenta_grupos_copias_y_bytes() {
        let (dir, mut lib) = library();
        let o = originals(&dir);
        for i in 0..2 {
            write_png(&o.join(format!("dos/{i}.png")), 1, 1, b"dos");
        }
        for i in 0..3 {
            write_png(&o.join(format!("tres/{i}.png")), 1, 1, b"tres!");
        }
        write_png(&o.join("sola.png"), 1, 1, b"sola");
        lib.index().unwrap();

        let size = |salt: &[u8]| 8 + 4 + 4 + 4 + 4 + 9 + salt.len() as u64;
        let s = lib.duplicate_summary().unwrap();
        assert_eq!(s.groups, 2);
        assert_eq!(s.extra_copies, 1 + 2);
        assert_eq!(s.extra_bytes, size(b"dos") + 2 * size(b"tres!"));
    }

    #[test]
    fn biblioteca_sin_duplicados() {
        let (dir, mut lib) = library();
        write_png(&originals(&dir).join("a.png"), 1, 1, b"a");
        lib.index().unwrap();
        assert_eq!(
            lib.duplicate_summary().unwrap(),
            DuplicateSummary::default()
        );
        let page = lib.duplicate_groups(None, 10).unwrap();
        assert!(page.groups.is_empty());
        assert!(page.next.is_none());
    }

    #[test]
    fn las_fotos_sin_huella_no_forman_grupo() {
        let (_dir, lib) = library();
        for rel in ["Originales/a.png", "Originales/b.png"] {
            lib.conn
                .execute(
                    "INSERT INTO photos (rel_path, size_bytes, modified_at, imported_at)
                     VALUES (?1, 10, 0, 0)",
                    [rel],
                )
                .unwrap();
        }
        assert_eq!(lib.duplicate_summary().unwrap().groups, 0);
        assert!(lib.duplicate_groups(None, 10).unwrap().groups.is_empty());
    }

    #[test]
    fn la_paginacion_devuelve_cada_grupo_una_vez_y_en_orden() {
        let (dir, mut lib) = library();
        let o = originals(&dir);
        // 7 grupos; dos pares comparten fecha para probar el desempate.
        for g in 0..7u64 {
            let mtime = 1_700_000_000 + (g / 2) * 86_400;
            for c in 0..2 {
                let p = o.join(format!("g{g}/c{c}.png"));
                write_png(&p, 1, 1, format!("grupo {g}").as_bytes());
                set_mtime(&p, mtime);
            }
        }
        lib.index().unwrap();

        let todos = all_groups(&lib, 50);
        assert_eq!(todos.len(), 7);
        for limit in [1, 2, 3] {
            assert_eq!(all_groups(&lib, limit), todos, "páginas de {limit}");
        }
        for w in todos.windows(2) {
            assert!(
                (w[0].date, &w[0].content_hash) > (w[1].date, &w[1].content_hash),
                "orden"
            );
        }
    }

    #[test]
    fn las_copias_van_por_orden_de_llegada_y_marcan_favorita_y_archivada() {
        let (dir, mut lib) = library();
        let o = originals(&dir);
        write_png(&o.join("b.png"), 1, 1, b"x");
        write_png(&o.join("a.png"), 1, 1, b"x");
        lib.index().unwrap();
        lib.conn
            .execute_batch(
                "UPDATE photos SET imported_at = 2 WHERE rel_path = 'Originales/a.png';
                 UPDATE photos SET imported_at = 1, favorite = 1, archived_at = 5
                     WHERE rel_path = 'Originales/b.png';",
            )
            .unwrap();

        let g = &lib.duplicate_groups(None, 10).unwrap().groups[0];
        assert_eq!(paths(g), ["Originales/b.png", "Originales/a.png"]);
        assert!(g.copies[0].favorite && g.copies[0].archived);
        assert!(!g.copies[1].favorite && !g.copies[1].archived);
    }

    #[test]
    fn una_copia_borrada_del_disco_sale_como_desaparecida() {
        let (dir, mut lib) = library();
        let o = originals(&dir);
        write_png(&o.join("antes/foto.png"), 1, 1, b"x");
        lib.index().unwrap();
        // Movida a mano fuera de la aplicación y vuelta a indexar.
        fs::create_dir_all(o.join("despues")).unwrap();
        fs::rename(o.join("antes/foto.png"), o.join("despues/foto.png")).unwrap();
        lib.index().unwrap();

        let g = &lib.duplicate_groups(None, 10).unwrap().groups[0];
        let estado: Vec<_> = g
            .copies
            .iter()
            .map(|c| (c.rel_path.as_str(), c.exists))
            .collect();
        assert_eq!(
            estado,
            [
                ("Originales/antes/foto.png", false),
                ("Originales/despues/foto.png", true)
            ]
        );
    }

    #[test]
    fn disco_desconectado_da_un_error_claro() {
        let (dir, mut lib) = library();
        write_png(&originals(&dir).join("a.png"), 1, 1, b"a");
        lib.index().unwrap();
        fs::remove_dir_all(originals(&dir)).unwrap();
        let err = lib.duplicate_groups(None, 10).unwrap_err().to_string();
        assert!(err.contains("conectado"), "{err}");
    }

    #[test]
    fn no_modifica_originales_ni_la_base_de_datos() {
        let (dir, mut lib) = library();
        let o = originals(&dir);
        write_png(&o.join("a.png"), 1, 1, b"x");
        write_png(&o.join("b.png"), 1, 1, b"x");
        lib.index().unwrap();

        let antes = snapshot(dir.path());
        lib.duplicate_summary().unwrap();
        all_groups(&lib, 1);
        assert_eq!(snapshot(dir.path()), antes);
    }
}
