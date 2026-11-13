//! Etiquetas (spec 016): palabras libres que el usuario pone a sus fotos.
//!
//! Una foto puede tener varias etiquetas y una etiqueta, muchas fotos. Solo
//! viven en la base de datos: nunca se escriben en los archivos de imagen.
//!
//! La columna `tags` del índice de búsqueda (`photo_search`) lleva los nombres
//! de las etiquetas de cada foto, para que la búsqueda de la spec 003 las
//! encuentre. Toda operación que cambia las etiquetas de una foto la rehace
//! dentro de la misma transacción.

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::library::Library;
use crate::timeline::DATE;

/// Longitud máxima del nombre de una etiqueta, en caracteres.
pub const MAX_TAG_NAME: usize = 50;

/// Una etiqueta con su número de fotos y la más reciente, para la portada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub photos: u64,
    /// Id de la foto más reciente de la etiqueta. `None` si no tiene fotos.
    pub cover: Option<i64>,
    /// Versión de edición de la portada, para pedir su miniatura editada.
    pub cover_edit_version: Option<u32>,
}

/// Nombre de etiqueta listo para guardar: sin espacios en los extremos, con
/// los de en medio reducidos a uno y sin caracteres de control. Entre 1 y
/// [`MAX_TAG_NAME`] caracteres.
pub fn normalize_tag_name(name: &str) -> Result<String> {
    let clean = name
        .split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let len = clean.chars().count();
    if len == 0 || len > MAX_TAG_NAME {
        return Err(Error::InvalidTagName(name.to_owned()));
    }
    Ok(clean)
}

/// Clave para comparar nombres sin distinguir mayúsculas (también fuera del
/// ASCII, que es lo único que cubre `COLLATE NOCASE`). Los acentos cuentan.
fn same_name_key(name: &str) -> String {
    name.to_lowercase()
}

impl Library {
    /// Todas las etiquetas, ordenadas por nombre sin distinguir mayúsculas.
    /// La interfaz las vuelve a ordenar con las reglas del idioma.
    pub fn tags(&self) -> Result<Vec<Tag>> {
        self.tags_where("", [])
    }

    /// Una etiqueta por su id.
    pub fn tag(&self, id: i64) -> Result<Tag> {
        self.tags_where("WHERE t.id = ?1", [id])?
            .pop()
            .ok_or(Error::TagNotFound(id))
    }

    fn tags_where<P: rusqlite::Params>(&self, condition: &str, params: P) -> Result<Vec<Tag>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT t.id, t.name,
                    (SELECT count(*) FROM photo_tags WHERE tag_id = t.id),
                    (SELECT p.id FROM photo_tags pt JOIN photos p ON p.id = pt.photo_id
                      WHERE pt.tag_id = t.id ORDER BY {DATE} DESC, p.id DESC LIMIT 1) AS cover
               FROM tags t {condition}
              ORDER BY t.name COLLATE NOCASE, t.id"
        ))?;
        let rows = stmt
            .query_map(params, |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut version = self
            .conn
            .prepare("SELECT edit_version FROM photos WHERE id = ?1")?;
        rows.into_iter()
            .map(|(id, name, photos, cover)| {
                let cover_edit_version = match cover {
                    Some(c) => version.query_row([c], |r| r.get(0))?,
                    None => None,
                };
                Ok(Tag {
                    id,
                    name,
                    photos: photos as u64,
                    cover,
                    cover_edit_version,
                })
            })
            .collect()
    }

    /// Crea la etiqueta `name`, o devuelve la que ya se llama así sin
    /// distinguir mayúsculas.
    pub fn create_tag(&mut self, name: &str) -> Result<Tag> {
        let name = normalize_tag_name(name)?;
        let id = match find_by_name(&self.conn, &name)? {
            Some(id) => id,
            None => {
                self.conn
                    .execute("INSERT INTO tags (name) VALUES (?1)", [&name])?;
                self.conn.last_insert_rowid()
            }
        };
        self.tag(id)
    }

    /// Cambia el nombre de la etiqueta `id`. Si ya hay otra con ese nombre,
    /// devuelve [`Error::TagNameTaken`] con su id y no cambia nada: la
    /// interfaz pregunta y, si el usuario quiere, llama a [`Library::merge_tags`].
    pub fn rename_tag(&mut self, id: i64, name: &str) -> Result<Tag> {
        let name = normalize_tag_name(name)?;
        let tx = self.conn.transaction()?;
        ensure_exists(&tx, id)?;
        if let Some(other) = find_by_name(&tx, &name)?
            && other != id
        {
            return Err(Error::TagNameTaken {
                name,
                existing: other,
            });
        }
        tx.execute("UPDATE tags SET name = ?2 WHERE id = ?1", params![id, name])?;
        let photos = photos_of(&tx, id)?;
        refresh_search(&tx, &photos)?;
        tx.commit()?;
        self.tag(id)
    }

    /// Une la etiqueta `from` con `into`: las fotos de `from` pasan a tener
    /// `into` y `from` desaparece. Devuelve `into` como queda.
    pub fn merge_tags(&mut self, from: i64, into: i64) -> Result<Tag> {
        if from == into {
            return self.tag(into);
        }
        let tx = self.conn.transaction()?;
        ensure_exists(&tx, from)?;
        ensure_exists(&tx, into)?;
        let photos = photos_of(&tx, from)?;
        tx.execute(
            "INSERT OR IGNORE INTO photo_tags (photo_id, tag_id)
             SELECT photo_id, ?2 FROM photo_tags WHERE tag_id = ?1",
            params![from, into],
        )?;
        tx.execute("DELETE FROM tags WHERE id = ?1", [from])?;
        refresh_search(&tx, &photos)?;
        tx.commit()?;
        self.tag(into)
    }

    /// Borra la etiqueta `id` y se la quita a sus fotos; las fotos no se
    /// tocan. Devuelve los ids de esas fotos para «Deshacer» (crear la
    /// etiqueta de nuevo y ponérsela).
    pub fn delete_tag(&mut self, id: i64) -> Result<Vec<i64>> {
        let tx = self.conn.transaction()?;
        ensure_exists(&tx, id)?;
        let photos = photos_of(&tx, id)?;
        // `photo_tags` se borra en cascada (`foreign_keys = ON`).
        tx.execute("DELETE FROM tags WHERE id = ?1", [id])?;
        refresh_search(&tx, &photos)?;
        tx.commit()?;
        Ok(photos)
    }
}

/// Cuántas fotos de una selección tienen cada etiqueta, para las casillas
/// del desplegable «Etiquetar»: todas, algunas o ninguna.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TagUsage {
    pub tag: i64,
    pub photos: u64,
}

impl Library {
    /// Pone la etiqueta `tag` a las fotos `ids` en una transacción. Devuelve
    /// las que no la tenían, en el orden recibido y sin repetir, para
    /// «Deshacer». Los ids que no existen se ignoran.
    pub fn add_tag(&mut self, ids: &[i64], tag: i64) -> Result<Vec<i64>> {
        self.change_tag(
            ids,
            tag,
            "INSERT OR IGNORE INTO photo_tags (photo_id, tag_id)
             SELECT id, ?2 FROM photos WHERE id = ?1",
        )
    }

    /// Quita la etiqueta `tag` a las fotos `ids` en una transacción. Devuelve
    /// las que la tenían, para «Deshacer».
    pub fn remove_tag(&mut self, ids: &[i64], tag: i64) -> Result<Vec<i64>> {
        self.change_tag(
            ids,
            tag,
            "DELETE FROM photo_tags WHERE photo_id = ?1 AND tag_id = ?2",
        )
    }

    fn change_tag(&mut self, ids: &[i64], tag: i64, sql: &str) -> Result<Vec<i64>> {
        let tx = self.conn.transaction()?;
        ensure_exists(&tx, tag)?;
        let mut changed = Vec::new();
        {
            // Una fila por sentencia, como en Favoritos: sin límite de
            // parámetros con selecciones de decenas de miles de fotos.
            let mut stmt = tx.prepare(sql)?;
            for &id in ids {
                if stmt.execute([id, tag])? > 0 {
                    changed.push(id);
                }
            }
        }
        refresh_search(&tx, &changed)?;
        tx.commit()?;
        Ok(changed)
    }

    /// Etiquetas de una foto, por nombre.
    pub fn photo_tags(&self, photo: i64) -> Result<Vec<Tag>> {
        self.tags_where(
            "WHERE t.id IN (SELECT tag_id FROM photo_tags WHERE photo_id = ?1)",
            [photo],
        )
    }

    /// Cuántas de las fotos `ids` tiene cada etiqueta. Solo salen las
    /// etiquetas que tiene alguna; las demás no las tiene ninguna.
    pub fn tag_usage(&self, ids: &[i64]) -> Result<Vec<TagUsage>> {
        // La selección va a una tabla temporal, que vive en memoria y no
        // toca el archivo de la biblioteca.
        self.conn.execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS tag_usage_ids (id INTEGER PRIMARY KEY);
             DELETE FROM temp.tag_usage_ids;",
        )?;
        {
            let mut insert = self
                .conn
                .prepare("INSERT OR IGNORE INTO temp.tag_usage_ids (id) VALUES (?1)")?;
            for &id in ids {
                insert.execute([id])?;
            }
        }
        let usage = self
            .conn
            .prepare(
                "SELECT pt.tag_id, count(*) FROM temp.tag_usage_ids s
                   JOIN photo_tags pt ON pt.photo_id = s.id
                  GROUP BY pt.tag_id ORDER BY pt.tag_id",
            )?
            .query_map([], |r| {
                Ok(TagUsage {
                    tag: r.get(0)?,
                    photos: r.get::<_, i64>(1)? as u64,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        self.conn.execute("DELETE FROM temp.tag_usage_ids", [])?;
        Ok(usage)
    }
}

/// Pasa a la foto `copy` las etiquetas de `original` (guardar como copia,
/// spec 009) y rehace su fila del índice de búsqueda.
pub(crate) fn copy_tags(tx: &Transaction<'_>, original: i64, copy: i64) -> Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO photo_tags (photo_id, tag_id)
         SELECT ?2, tag_id FROM photo_tags WHERE photo_id = ?1",
        params![original, copy],
    )?;
    refresh_search(tx, &[copy])
}

fn find_by_name(conn: &Connection, name: &str) -> Result<Option<i64>> {
    // Lo normal es que coincida tal cual o en ASCII sin mayúsculas: lo
    // resuelve el índice único. Si no, se comparan todos los nombres en
    // minúsculas (son pocos).
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
            [name],
            |r| r.get(0),
        )
        .optional()?
    {
        return Ok(Some(id));
    }
    let key = same_name_key(name);
    let mut stmt = conn.prepare("SELECT id, name FROM tags")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let other: String = row.get(1)?;
        if same_name_key(&other) == key {
            return Ok(Some(row.get(0)?));
        }
    }
    Ok(None)
}

fn ensure_exists(conn: &Connection, id: i64) -> Result<()> {
    conn.query_row("SELECT 1 FROM tags WHERE id = ?1", [id], |_| Ok(()))
        .optional()?
        .ok_or(Error::TagNotFound(id))
}

fn photos_of(conn: &Connection, tag: i64) -> Result<Vec<i64>> {
    let mut stmt =
        conn.prepare("SELECT photo_id FROM photo_tags WHERE tag_id = ?1 ORDER BY photo_id")?;
    let ids = stmt
        .query_map([tag], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(ids)
}

/// Rehace la columna `tags` del índice de búsqueda de las fotos `ids`.
pub(crate) fn refresh_search(tx: &Transaction<'_>, ids: &[i64]) -> Result<()> {
    let mut stmt = tx.prepare(
        "UPDATE photo_search
            SET tags = coalesce((SELECT group_concat(t.name, ' ')
                                   FROM photo_tags pt JOIN tags t ON t.id = pt.tag_id
                                  WHERE pt.photo_id = ?1), '')
          WHERE rowid = ?1",
    )?;
    for &id in ids {
        stmt.execute([id])?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::search::SearchQuery;

    /// Biblioteca con `n` fotos (ids 1..=n, la n la más reciente) y su fila
    /// en el índice de búsqueda.
    pub(crate) fn library(n: i64) -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        let tx = lib.conn.transaction().unwrap();
        for i in 1..=n {
            let rel = format!("Originales/{i}.jpg");
            tx.execute(
                "INSERT INTO photos (id, rel_path, size_bytes, modified_at, imported_at)
                     VALUES (?1, ?2, 1, ?1, 0)",
                params![i, rel],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO photo_search (rowid, rel_path, tags) VALUES (?1, ?2, '')",
                params![i, rel],
            )
            .unwrap();
        }
        tx.commit().unwrap();
        (dir, lib)
    }

    pub(crate) fn tag_photos(lib: &mut Library, tag: i64, ids: &[i64]) {
        let tx = lib.conn.transaction().unwrap();
        for &id in ids {
            tx.execute(
                "INSERT OR IGNORE INTO photo_tags (photo_id, tag_id) VALUES (?1, ?2)",
                [id, tag],
            )
            .unwrap();
        }
        refresh_search(&tx, ids).unwrap();
        tx.commit().unwrap();
    }

    pub(crate) fn found(lib: &Library, text: &str) -> Vec<i64> {
        let mut ids: Vec<i64> = lib
            .search_page(&SearchQuery::parse(text), None, 1000)
            .unwrap()
            .photos
            .iter()
            .map(|p| p.id)
            .collect();
        ids.sort();
        ids
    }

    fn names(lib: &Library) -> Vec<String> {
        lib.tags().unwrap().into_iter().map(|t| t.name).collect()
    }

    #[test]
    fn normaliza_los_nombres() {
        assert_eq!(
            normalize_tag_name("  Ruta   del\tCares ").unwrap(),
            "Ruta del Cares"
        );
        assert_eq!(normalize_tag_name("a\u{0}b").unwrap(), "a b");
        assert_eq!(normalize_tag_name("🏖️ \"AND\" *").unwrap(), "🏖️ \"AND\" *");
        assert!(normalize_tag_name("   ").is_err());
        assert!(normalize_tag_name("").is_err());
        assert!(normalize_tag_name(&"ñ".repeat(50)).is_ok());
        assert!(normalize_tag_name(&"ñ".repeat(51)).is_err());
    }

    #[test]
    fn crear_reutiliza_la_que_ya_existe_sin_mirar_mayusculas() {
        let (_dir, mut lib) = library(0);
        let playa = lib.create_tag("Playa").unwrap();
        assert_eq!(lib.create_tag(" playa ").unwrap().id, playa.id);
        let avila = lib.create_tag("Ávila").unwrap();
        assert_eq!(lib.create_tag("ÁVILA").unwrap().id, avila.id);
        // Los acentos cuentan.
        let otra = lib.create_tag("Avila").unwrap();
        assert_ne!(otra.id, avila.id);
        // El orden fino (la «Á» junto a la «A») lo pone la interfaz.
        let mut todas = names(&lib);
        todas.sort();
        assert_eq!(todas, ["Avila", "Playa", "Ávila"]);
    }

    #[test]
    fn lista_con_recuento_y_la_foto_mas_reciente_de_portada() {
        let (_dir, mut lib) = library(5);
        let playa = lib.create_tag("Playa").unwrap();
        let vacia = lib.create_tag("abuela").unwrap();
        tag_photos(&mut lib, playa.id, &[1, 4, 2]);
        lib.conn
            .execute("UPDATE photos SET edit_version = 3 WHERE id = 4", [])
            .unwrap();

        let tags = lib.tags().unwrap();
        assert_eq!(
            tags,
            [
                Tag {
                    id: vacia.id,
                    name: "abuela".into(),
                    photos: 0,
                    cover: None,
                    cover_edit_version: None,
                },
                Tag {
                    id: playa.id,
                    name: "Playa".into(),
                    photos: 3,
                    cover: Some(4),
                    cover_edit_version: Some(3),
                },
            ]
        );
    }

    #[test]
    fn renombrar_y_avisar_si_el_nombre_ya_es_de_otra() {
        let (_dir, mut lib) = library(2);
        let playa = lib.create_tag("Playa").unwrap();
        let mar = lib.create_tag("Mar").unwrap();
        tag_photos(&mut lib, playa.id, &[1]);

        // Cambiar solo las mayúsculas de la propia etiqueta vale.
        assert_eq!(lib.rename_tag(playa.id, "PLAYA").unwrap().name, "PLAYA");
        assert_eq!(found(&lib, "playa"), [1]);

        match lib.rename_tag(playa.id, "mar") {
            Err(Error::TagNameTaken { existing, .. }) => assert_eq!(existing, mar.id),
            other => panic!("se esperaba TagNameTaken: {other:?}"),
        }
        assert_eq!(names(&lib), ["Mar", "PLAYA"]);

        assert_eq!(lib.rename_tag(playa.id, "Costa").unwrap().name, "Costa");
        assert_eq!(found(&lib, "costa"), [1]);
        assert!(found(&lib, "playa").is_empty());

        assert!(matches!(
            lib.rename_tag(999, "x"),
            Err(Error::TagNotFound(999))
        ));
        assert!(matches!(
            lib.rename_tag(mar.id, " "),
            Err(Error::InvalidTagName(_))
        ));
    }

    #[test]
    fn unir_pasa_las_fotos_y_quita_la_primera() {
        let (_dir, mut lib) = library(4);
        let playa = lib.create_tag("Playa").unwrap();
        let mar = lib.create_tag("Mar").unwrap();
        tag_photos(&mut lib, playa.id, &[1, 2]);
        tag_photos(&mut lib, mar.id, &[2, 3]);

        let unida = lib.merge_tags(playa.id, mar.id).unwrap();
        assert_eq!((unida.id, unida.photos), (mar.id, 3));
        assert_eq!(names(&lib), ["Mar"]);
        assert_eq!(found(&lib, "mar"), [1, 2, 3]);
        assert!(found(&lib, "playa").is_empty());
        assert_eq!(lib.merge_tags(mar.id, mar.id).unwrap().photos, 3);
    }

    #[test]
    fn borrar_quita_la_etiqueta_y_devuelve_sus_fotos() {
        let (_dir, mut lib) = library(3);
        let playa = lib.create_tag("Playa").unwrap();
        tag_photos(&mut lib, playa.id, &[3, 1]);
        assert_eq!(found(&lib, "playa"), [1, 3]);

        assert_eq!(lib.delete_tag(playa.id).unwrap(), [1, 3]);
        assert!(lib.tags().unwrap().is_empty());
        assert!(found(&lib, "playa").is_empty());
        let filas: i64 = lib
            .conn
            .query_row("SELECT count(*) FROM photo_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(filas, 0);
        let fotos: i64 = lib
            .conn
            .query_row("SELECT count(*) FROM photos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fotos, 3);
        assert!(matches!(
            lib.delete_tag(playa.id),
            Err(Error::TagNotFound(_))
        ));
    }

    #[test]
    fn nombres_raros_no_rompen_la_busqueda() {
        let (_dir, mut lib) = library(1);
        for raro in ["\"AND\"", "NEAR(", "*", "🏖️", "a/b", "c'est"] {
            let tag = lib.create_tag(raro).unwrap();
            tag_photos(&mut lib, tag.id, &[1]);
            lib.search_page(&SearchQuery::parse(raro), None, 10)
                .unwrap();
        }
        assert_eq!(found(&lib, "c'est"), [1]);
    }

    #[test]
    fn poner_y_quitar_devuelve_lo_que_cambia_y_deshacer_lo_deja_igual() {
        let (_dir, mut lib) = library(5);
        let playa = lib.create_tag("Playa").unwrap();
        assert_eq!(lib.add_tag(&[1], playa.id).unwrap(), [1]);
        // Selección mixta: la 1 ya la tenía, la 99 no existe y la 3 va repetida.
        let puestas = lib.add_tag(&[1, 3, 99, 3, 2], playa.id).unwrap();
        assert_eq!(puestas, [3, 2]);
        assert_eq!(found(&lib, "playa"), [1, 2, 3]);

        // Deshacer quita solo las que se pusieron.
        assert_eq!(lib.remove_tag(&puestas, playa.id).unwrap(), [3, 2]);
        assert_eq!(found(&lib, "playa"), [1]);
        assert!(lib.remove_tag(&[4, 5], playa.id).unwrap().is_empty());
        assert!(matches!(
            lib.add_tag(&[1], 999),
            Err(Error::TagNotFound(999))
        ));
    }

    #[test]
    fn la_busqueda_combina_ruta_etiquetas_y_fecha() {
        let (_dir, mut lib) = library(3);
        let playa = lib.create_tag("Playa de Málaga").unwrap();
        let abuela = lib.create_tag("Abuela").unwrap();
        lib.add_tag(&[1, 2], playa.id).unwrap();
        lib.add_tag(&[2], abuela.id).unwrap();
        assert_eq!(found(&lib, "malaga"), [1, 2]);
        assert_eq!(found(&lib, "PLA abu"), [2]);
        assert_eq!(found(&lib, "originales abuela"), [2]);
    }

    #[test]
    fn etiquetas_de_una_foto_y_uso_en_una_seleccion() {
        let (_dir, mut lib) = library(4);
        let playa = lib.create_tag("Playa").unwrap();
        let abuela = lib.create_tag("Abuela").unwrap();
        lib.create_tag("Gatos").unwrap();
        lib.add_tag(&[1, 2, 3], playa.id).unwrap();
        lib.add_tag(&[2], abuela.id).unwrap();

        let de_la_2: Vec<String> = lib
            .photo_tags(2)
            .unwrap()
            .into_iter()
            .map(|t| t.name)
            .collect();
        assert_eq!(de_la_2, ["Abuela", "Playa"]);
        assert!(lib.photo_tags(4).unwrap().is_empty());

        let uso = lib.tag_usage(&[2, 3, 4, 2]).unwrap();
        assert_eq!(
            uso,
            [
                TagUsage {
                    tag: playa.id,
                    photos: 2
                },
                TagUsage {
                    tag: abuela.id,
                    photos: 1
                },
            ]
        );
        assert!(lib.tag_usage(&[]).unwrap().is_empty());
        // Otra selección no arrastra la anterior.
        assert_eq!(
            lib.tag_usage(&[1]).unwrap(),
            [TagUsage {
                tag: playa.id,
                photos: 1
            }]
        );
    }

    #[test]
    fn si_falla_a_mitad_no_cambia_ninguna_foto() {
        let (_dir, mut lib) = library(5);
        let playa = lib.create_tag("Playa").unwrap();
        lib.conn
            .execute_batch(
                "CREATE TRIGGER fallo BEFORE INSERT ON photo_tags WHEN NEW.photo_id = 4
                 BEGIN SELECT RAISE(ABORT, 'disco desconectado'); END;",
            )
            .unwrap();
        assert!(lib.add_tag(&[1, 2, 3, 4, 5], playa.id).is_err());
        assert_eq!(lib.tag(playa.id).unwrap().photos, 0);
        assert!(found(&lib, "playa").is_empty());
    }

    #[test]
    fn selecciones_enormes() {
        let (_dir, mut lib) = library(40_000);
        let playa = lib.create_tag("Playa").unwrap();
        let todas: Vec<i64> = (1..=40_000).collect();
        assert_eq!(lib.add_tag(&todas, playa.id).unwrap().len(), 40_000);
        assert_eq!(
            lib.tag_usage(&todas).unwrap(),
            [TagUsage {
                tag: playa.id,
                photos: 40_000
            }]
        );
        assert_eq!(lib.tag(playa.id).unwrap().cover, Some(40_000));
    }
}
