//! Álbumes (spec 014): listas de fotos con nombre, como «Picos de Europa».
//!
//! Un álbum solo vive en la base de datos (`albums` y `album_photos`).
//! Crearlo, renombrarlo o eliminarlo nunca toca los archivos de imagen, y
//! eliminarlo no quita ninguna foto de la biblioteca.

use std::time::SystemTime;

use rusqlite::OptionalExtension;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::files::unix_secs;
use crate::library::Library;

/// Longitud máxima del nombre de un álbum, en caracteres.
pub const MAX_ALBUM_NAME: usize = 100;

/// Un álbum tal como lo enseña la pantalla Álbumes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Album {
    pub id: i64,
    pub name: String,
    /// Número de fotos, archivadas incluidas y sin las de la Papelera.
    pub photos: u64,
    /// La foto más reciente del álbum, que hace de portada. `None` si está
    /// vacío.
    pub cover: Option<i64>,
    /// Versión de la edición que se ve de la portada (spec 009), para pedir
    /// su miniatura editada.
    pub cover_edit_version: Option<u32>,
    /// Fecha (segundos Unix) de la foto más antigua y de la más reciente,
    /// para el rango de la cabecera. `None` si está vacío.
    pub first: Option<i64>,
    pub last: Option<i64>,
    /// Última vez que se creó, renombró o cambió (segundos Unix).
    pub updated_at: i64,
}

/// Columnas y uniones comunes a `albums` y `album`. La portada es la foto con
/// la fecha más reciente, con el mismo orden que la cuadrícula.
const SELECT: &str = "
    SELECT a.id, a.name, count(p.id),
           (SELECT p2.id FROM album_photos ap2
                JOIN photos p2 ON p2.id = ap2.photo_id
            WHERE ap2.album_id = a.id AND p2.trashed_at IS NULL
            ORDER BY coalesce(p2.taken_at, p2.modified_at) DESC, p2.id DESC
            LIMIT 1),
           min(coalesce(p.taken_at, p.modified_at)),
           max(coalesce(p.taken_at, p.modified_at)),
           a.updated_at,
           (SELECT p2.edit_version FROM album_photos ap2
                JOIN photos p2 ON p2.id = ap2.photo_id
            WHERE ap2.album_id = a.id AND p2.trashed_at IS NULL
            ORDER BY coalesce(p2.taken_at, p2.modified_at) DESC, p2.id DESC
            LIMIT 1)
    FROM albums a
         LEFT JOIN album_photos ap ON ap.album_id = a.id
         LEFT JOIN photos p ON p.id = ap.photo_id AND p.trashed_at IS NULL";

fn album_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Album> {
    Ok(Album {
        id: r.get(0)?,
        name: r.get(1)?,
        photos: r.get(2)?,
        cover: r.get(3)?,
        first: r.get(4)?,
        last: r.get(5)?,
        updated_at: r.get(6)?,
        cover_edit_version: r.get(7)?,
    })
}

impl Library {
    /// Todos los álbumes, del modificado más recientemente al más antiguo.
    /// Son pocos (los crea el usuario), así que la interfaz los pide de una
    /// vez.
    pub fn albums(&self) -> Result<Vec<Album>> {
        let mut stmt = self.conn.prepare(&format!(
            "{SELECT} GROUP BY a.id ORDER BY a.updated_at DESC, a.id DESC"
        ))?;
        let albums = stmt
            .query_map([], album_from_row)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(albums)
    }

    /// Un álbum, o `None` si no existe.
    pub fn album(&self, id: i64) -> Result<Option<Album>> {
        Ok(self
            .conn
            .query_row(
                &format!("{SELECT} WHERE a.id = ?1 GROUP BY a.id"),
                [id],
                album_from_row,
            )
            .optional()?)
    }

    /// Crea un álbum vacío y devuelve su id. El nombre se normaliza como en
    /// [`Library::rename_album`]. Puede haber dos álbumes con el mismo nombre.
    pub fn create_album(&self, name: &str) -> Result<i64> {
        let name = normalize(name)?;
        let now = unix_secs(SystemTime::now());
        self.conn.execute(
            "INSERT INTO albums (name, created_at, updated_at) VALUES (?1, ?2, ?2)",
            (&name, now),
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Cambia el nombre del álbum `id` y devuelve cómo ha quedado. Los saltos
    /// de línea y tabuladores pasan a ser espacios, se quitan los espacios de
    /// los extremos y se corta a [`MAX_ALBUM_NAME`] caracteres. Un nombre
    /// vacío es un error.
    pub fn rename_album(&self, id: i64, name: &str) -> Result<String> {
        let name = normalize(name)?;
        let rows = self.conn.execute(
            "UPDATE albums SET name = ?2, updated_at = ?3 WHERE id = ?1",
            (id, &name, unix_secs(SystemTime::now())),
        )?;
        if rows == 0 {
            return Err(Error::AlbumNotFound(id));
        }
        Ok(name)
    }

    /// Elimina el álbum `id`. Sus fotos siguen en la biblioteca: solo se
    /// borran sus filas de `album_photos` (por `ON DELETE CASCADE`).
    pub fn delete_album(&self, id: i64) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM albums WHERE id = ?1", [id])?;
        if rows == 0 {
            return Err(Error::AlbumNotFound(id));
        }
        Ok(())
    }

    /// Añade las fotos `ids` al final del álbum `album` en una transacción.
    /// Devuelve las que no estaban, en el orden recibido y sin repetir. Los
    /// ids que no existen se ignoran. Si cambia algo, el álbum sube arriba
    /// en la pantalla Álbumes.
    pub fn add_to_album(&mut self, album: i64, ids: &[i64]) -> Result<Vec<i64>> {
        self.change_album(album, ids, true)
    }

    /// Quita las fotos `ids` del álbum `album` (no de la biblioteca) en una
    /// transacción. Devuelve las que estaban, para «Deshacer».
    pub fn remove_from_album(&mut self, album: i64, ids: &[i64]) -> Result<Vec<i64>> {
        self.change_album(album, ids, false)
    }

    fn change_album(&mut self, album: i64, ids: &[i64], add: bool) -> Result<Vec<i64>> {
        let tx = self.conn.transaction()?;
        let exists = tx
            .query_row("SELECT 1 FROM albums WHERE id = ?1", [album], |_| Ok(()))
            .optional()?
            .is_some();
        if !exists {
            return Err(Error::AlbumNotFound(album));
        }
        let mut changed = Vec::new();
        {
            // Una fila por sentencia, como en Favoritos y Etiquetas: sin
            // límite de parámetros con selecciones de decenas de miles de
            // fotos. Las nuevas van al final, numeradas aquí para no buscar
            // la última posición en cada fila.
            let mut next: i64 = tx.query_row(
                "SELECT coalesce(max(position) + 1, 0) FROM album_photos WHERE album_id = ?1",
                [album],
                |r| r.get(0),
            )?;
            let mut insert = tx.prepare(
                "INSERT OR IGNORE INTO album_photos (album_id, photo_id, position)
                 SELECT ?1, id, ?3 FROM photos WHERE id = ?2",
            )?;
            let mut delete =
                tx.prepare("DELETE FROM album_photos WHERE album_id = ?1 AND photo_id = ?2")?;
            for &id in ids {
                let rows = if add {
                    insert.execute([album, id, next])?
                } else {
                    delete.execute([album, id])?
                };
                if rows > 0 {
                    changed.push(id);
                    next += 1;
                }
            }
        }
        if !changed.is_empty() {
            tx.execute(
                "UPDATE albums SET updated_at = ?2 WHERE id = ?1",
                (album, unix_secs(SystemTime::now())),
            )?;
        }
        tx.commit()?;
        Ok(changed)
    }
}

fn normalize(name: &str) -> Result<String> {
    let flat: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let name = flat
        .trim()
        .chars()
        .take(MAX_ALBUM_NAME)
        .collect::<String>()
        .trim_end()
        .to_owned();
    if name.is_empty() {
        return Err(Error::EmptyAlbumName);
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::View;
    use rusqlite::params;

    /// Biblioteca con `n` fotos; la foto `i` tiene fecha `i * 100`.
    fn library(n: i64) -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        let tx = lib.conn.transaction().unwrap();
        for i in 1..=n {
            tx.execute(
                "INSERT INTO photos (id, rel_path, size_bytes, modified_at, imported_at)
                     VALUES (?1, ?2, 1, ?1 * 100, 0)",
                params![i, format!("Originales/{i}.jpg")],
            )
            .unwrap();
        }
        tx.commit().unwrap();
        (dir, lib)
    }

    fn put(lib: &Library, album: i64, photos: &[i64]) {
        for &p in photos {
            lib.conn
                .execute(
                    "INSERT INTO album_photos (album_id, photo_id) VALUES (?1, ?2)",
                    (album, p),
                )
                .unwrap();
        }
    }

    fn set_updated(lib: &Library, album: i64, at: i64) {
        lib.conn
            .execute(
                "UPDATE albums SET updated_at = ?2 WHERE id = ?1",
                (album, at),
            )
            .unwrap();
    }

    fn ids(lib: &Library, album: i64) -> Vec<i64> {
        let pagina = lib.view_page(View::Album(album), None, 100).unwrap();
        pagina.photos.iter().map(|p| p.id).collect()
    }

    #[test]
    fn anade_y_quita_fotos() {
        let (_dir, mut lib) = library(5);
        let a = lib.create_album("A").unwrap();
        set_updated(&lib, a, 1);
        assert_eq!(lib.add_to_album(a, &[2, 4, 4, 999]).unwrap(), [2, 4]);
        assert!(lib.album(a).unwrap().unwrap().updated_at > 1);
        // Las que ya estaban no se repiten ni cuentan.
        assert_eq!(lib.add_to_album(a, &[4, 1]).unwrap(), [1]);
        assert_eq!(ids(&lib, a), [4, 2, 1]);
        let posiciones: Vec<i64> = lib
            .conn
            .prepare("SELECT position FROM album_photos WHERE album_id = ?1 ORDER BY photo_id")
            .unwrap()
            .query_map([a], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(posiciones, [2, 0, 1]);

        set_updated(&lib, a, 1);
        assert_eq!(lib.remove_from_album(a, &[2, 3]).unwrap(), [2]);
        assert!(lib.album(a).unwrap().unwrap().updated_at > 1);
        assert_eq!(ids(&lib, a), [4, 1]);
        // Quitar no borra la foto.
        assert!(lib.view_page(View::Photos, None, 100).unwrap().photos.len() == 5);

        // Sin cambios, el álbum no sube.
        set_updated(&lib, a, 1);
        assert!(lib.remove_from_album(a, &[2]).unwrap().is_empty());
        assert_eq!(lib.album(a).unwrap().unwrap().updated_at, 1);
    }

    #[test]
    fn anade_y_quita_10000_fotos_en_menos_de_un_segundo() {
        let (_dir, mut lib) = library(10_000);
        let a = lib.create_album("Todo").unwrap();
        let todas: Vec<i64> = (1..=10_000).collect();
        let inicio = std::time::Instant::now();
        assert_eq!(lib.add_to_album(a, &todas).unwrap().len(), 10_000);
        let anadir = inicio.elapsed();
        let inicio = std::time::Instant::now();
        assert_eq!(lib.remove_from_album(a, &todas).unwrap().len(), 10_000);
        let quitar = inicio.elapsed();
        // Holgado para el modo de depuración; en release es mucho menos.
        assert!(
            anadir.max(quitar) < std::time::Duration::from_secs(1),
            "{anadir:?} {quitar:?}"
        );
    }

    #[test]
    fn anadir_a_un_album_que_no_existe_es_un_error() {
        let (_dir, mut lib) = library(2);
        assert!(matches!(
            lib.add_to_album(7, &[1]),
            Err(Error::AlbumNotFound(7))
        ));
        assert!(matches!(
            lib.remove_from_album(7, &[1]),
            Err(Error::AlbumNotFound(7))
        ));
    }

    #[test]
    fn la_vista_de_un_album_incluye_las_archivadas() {
        use crate::selection::Scope;

        let (_dir, mut lib) = library(6);
        let a = lib.create_album("A").unwrap();
        let b = lib.create_album("B").unwrap();
        lib.add_to_album(a, &[1, 3, 4, 6]).unwrap();
        lib.add_to_album(b, &[2]).unwrap();
        lib.set_archived(&[4], true).unwrap();

        let vista = View::Album(a);
        let total: u64 = lib
            .view_months(vista)
            .unwrap()
            .iter()
            .map(|m| m.photos)
            .sum();
        assert_eq!(total, 4);
        let primera = lib.view_page(vista, None, 2).unwrap();
        let ids: Vec<i64> = primera.photos.iter().map(|p| p.id).collect();
        assert_eq!(ids, [6, 4]);
        assert!(primera.photos[1].archived);
        let segunda = lib.view_page(vista, primera.next, 2).unwrap();
        let ids: Vec<i64> = segunda.photos.iter().map(|p| p.id).collect();
        assert_eq!(ids, [3, 1]);
        assert!(segunda.next.is_none());
        assert_eq!(lib.select_all(Scope::View(vista)).unwrap(), [6, 4, 3, 1]);
        assert!(
            lib.view_page(View::Album(999), None, 10)
                .unwrap()
                .photos
                .is_empty()
        );
    }

    #[test]
    fn la_vista_de_un_album_viaja_como_objeto() {
        let v: View = serde_json::from_str(r#"{"album": 7}"#).unwrap();
        assert_eq!(v, View::Album(7));
        assert_eq!(
            serde_json::to_string(&View::Album(7)).unwrap(),
            r#"{"album":7}"#
        );
    }

    #[test]
    fn crea_un_album_vacio() {
        let (_dir, lib) = library(0);
        let id = lib.create_album("  Picos de Europa ").unwrap();
        let album = lib.album(id).unwrap().unwrap();
        assert_eq!(album.name, "Picos de Europa");
        assert_eq!(
            (album.photos, album.cover, album.first, album.last),
            (0, None, None, None)
        );
        assert!(album.updated_at > 0);
        assert_eq!(lib.albums().unwrap(), [album]);
    }

    #[test]
    fn portada_numero_y_rango_de_fechas() {
        let (_dir, lib) = library(5);
        let id = lib.create_album("Viaje").unwrap();
        put(&lib, id, &[2, 4, 3]);
        // Archivar no saca la foto del álbum ni la descuenta.
        lib.conn
            .execute("UPDATE photos SET archived_at = 1 WHERE id = 4", [])
            .unwrap();
        let album = lib.album(id).unwrap().unwrap();
        assert_eq!(album.photos, 3);
        assert_eq!(album.cover, Some(4));
        assert_eq!(album.cover_edit_version, None);
        assert_eq!((album.first, album.last), (Some(200), Some(400)));
        lib.conn
            .execute("UPDATE photos SET edit_version = 2 WHERE id = 4", [])
            .unwrap();
        assert_eq!(lib.album(id).unwrap().unwrap().cover_edit_version, Some(2));

        // La fecha de captura manda sobre la de modificación.
        lib.conn
            .execute("UPDATE photos SET taken_at = 50 WHERE id = 4", [])
            .unwrap();
        let album = lib.album(id).unwrap().unwrap();
        assert_eq!(album.cover, Some(3));
        assert_eq!((album.first, album.last), (Some(50), Some(300)));
    }

    #[test]
    fn ordena_por_ultima_modificacion() {
        let (_dir, lib) = library(0);
        let a = lib.create_album("A").unwrap();
        let b = lib.create_album("B").unwrap();
        let c = lib.create_album("C").unwrap();
        set_updated(&lib, a, 30);
        set_updated(&lib, b, 10);
        set_updated(&lib, c, 20);
        let ids: Vec<i64> = lib.albums().unwrap().iter().map(|a| a.id).collect();
        assert_eq!(ids, [a, c, b]);

        // Renombrar lo sube arriba.
        lib.rename_album(b, "B2").unwrap();
        assert_eq!(lib.albums().unwrap()[0].id, b);
    }

    #[test]
    fn renombra_y_normaliza_el_nombre() {
        let (_dir, lib) = library(0);
        let id = lib.create_album("Boda").unwrap();
        assert_eq!(
            lib.rename_album(id, "Boda de\tAna\n y Luis ").unwrap(),
            "Boda de Ana  y Luis"
        );
        let largo = "🏔️".repeat(80);
        let nombre = lib.rename_album(id, &largo).unwrap();
        assert_eq!(nombre.chars().count(), MAX_ALBUM_NAME);
        assert_eq!(lib.album(id).unwrap().unwrap().name, nombre);
    }

    #[test]
    fn rechaza_nombres_vacios_y_albumes_que_no_existen() {
        let (_dir, lib) = library(0);
        assert!(matches!(
            lib.create_album(" \n "),
            Err(Error::EmptyAlbumName)
        ));
        let id = lib.create_album("Gatos").unwrap();
        assert!(matches!(
            lib.rename_album(id, ""),
            Err(Error::EmptyAlbumName)
        ));
        assert_eq!(lib.album(id).unwrap().unwrap().name, "Gatos");
        assert!(matches!(
            lib.rename_album(999, "Perros"),
            Err(Error::AlbumNotFound(999))
        ));
        assert!(matches!(
            lib.delete_album(999),
            Err(Error::AlbumNotFound(999))
        ));
        assert_eq!(lib.album(999).unwrap(), None);
    }

    #[test]
    fn nombres_repetidos_permitidos() {
        let (_dir, lib) = library(0);
        let a = lib.create_album("Navidad").unwrap();
        let b = lib.create_album("Navidad").unwrap();
        assert_ne!(a, b);
        assert_eq!(lib.albums().unwrap().len(), 2);
    }

    #[test]
    fn eliminar_no_borra_fotos() {
        let (_dir, lib) = library(3);
        let a = lib.create_album("A").unwrap();
        let b = lib.create_album("B").unwrap();
        put(&lib, a, &[1, 2]);
        put(&lib, b, &[2, 3]);
        lib.delete_album(a).unwrap();
        assert_eq!(lib.album(a).unwrap(), None);
        let fotos: i64 = lib
            .conn
            .query_row("SELECT count(*) FROM photos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fotos, 3);
        let filas: i64 = lib
            .conn
            .query_row("SELECT count(*) FROM album_photos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(filas, 2);
        assert_eq!(lib.album(b).unwrap().unwrap().photos, 2);
    }

    #[test]
    fn las_fotos_de_la_papelera_no_cuentan_ni_se_ven() {
        let (_dir, mut lib) = library(3);
        let a = lib.create_album("A").unwrap();
        lib.add_to_album(a, &[1, 2, 3]).unwrap();
        lib.conn
            .execute("UPDATE photos SET trashed_at = 1 WHERE id = 3", [])
            .unwrap();
        let album = lib.album(a).unwrap().unwrap();
        assert_eq!(
            (album.photos, album.cover, album.last),
            (2, Some(2), Some(200))
        );
        assert_eq!(ids(&lib, a), [2, 1]);
    }

    #[test]
    fn borrar_una_foto_la_saca_de_sus_albumes() {
        let (_dir, lib) = library(3);
        let a = lib.create_album("A").unwrap();
        put(&lib, a, &[1, 3]);
        lib.conn
            .execute("DELETE FROM photos WHERE id = 3", [])
            .unwrap();
        let album = lib.album(a).unwrap().unwrap();
        assert_eq!((album.photos, album.cover), (1, Some(1)));
    }
}
