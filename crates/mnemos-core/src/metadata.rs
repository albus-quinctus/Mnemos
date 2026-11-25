//! Valoración, descripción, palabras clave, cámara y objetivo leídos de los
//! archivos (spec 024).
//!
//! Se leen una sola vez por foto (`metadata_at`), como el `.json` de Takeout:
//! si después el usuario cambia las estrellas o quita una etiqueta, reindexar
//! no lo deshace. Las estrellas puestas a mano (`rating_source = 'manual'`) no
//! se pisan nunca. Nada se escribe en los archivos.

use std::path::Path;

use mnemos_imaging::XmpData;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::library::Library;
use crate::paths::RelPath;
use crate::tags::{find_by_name, normalize_tag_name, refresh_search};

/// Lo que se guarda de un archivo. Cada campo falta si el archivo no lo trae.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FileMeta {
    pub rating: Option<u8>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub camera: Option<String>,
    pub lens: Option<String>,
}

impl FileMeta {
    /// Lee el XMP incrustado, la valoración EXIF y la cámara de `path`. La
    /// valoración del XMP gana a la del EXIF. Solo lee el archivo.
    pub(crate) fn of_file(path: &Path) -> Self {
        let xmp = if mnemos_imaging::is_video(path) {
            XmpData::default()
        } else {
            mnemos_imaging::read_xmp(path).unwrap_or_default()
        };
        let rating = xmp.rating.or_else(|| {
            (!mnemos_imaging::is_video(path))
                .then(|| mnemos_imaging::exif_rating(path))
                .flatten()
        });
        let camera = mnemos_imaging::camera_info(path).unwrap_or_default();
        FileMeta {
            rating,
            description: xmp.description,
            keywords: xmp.keywords,
            camera: camera.camera,
            lens: camera.lens,
        }
    }

    /// Lo del `.xmp` junto al archivo de origen gana a lo del archivo, campo
    /// a campo, como en Lightroom o darktable. Las palabras clave se suman,
    /// primero las del `.xmp`.
    pub(crate) fn with_sidecar(mut self, sidecar: XmpData) -> Self {
        self.rating = sidecar.rating.or(self.rating);
        self.description = sidecar.description.or(self.description);
        let mut keywords = sidecar.keywords;
        for k in self.keywords {
            if !keywords
                .iter()
                .any(|o| o.to_lowercase() == k.to_lowercase())
            {
                keywords.push(k);
            }
        }
        self.keywords = keywords;
        self
    }
}

/// `.xmp` junto a `src`: `foto.CR2.xmp` (darktable, digiKam) y, si no existe,
/// `foto.xmp` (Lightroom, Bridge), con la extensión en minúsculas o
/// mayúsculas. Solo lee.
pub(crate) fn sidecar_of(src: &Path) -> Option<XmpData> {
    let dir = src.parent()?;
    let name = src.file_name()?.to_str()?;
    let stem = src.file_stem()?.to_str()?;
    [name, stem]
        .iter()
        .flat_map(|base| ["xmp", "XMP"].map(|ext| dir.join(format!("{base}.{ext}"))))
        .find(|p| p.is_file())
        .and_then(|p| mnemos_imaging::read_xmp_sidecar(&p))
}

/// Guarda `meta` en la foto `id` si todavía no se leyó (`metadata_at` es
/// NULL). La descripción solo se pone si estaba vacía (la de Takeout gana) y
/// la valoración manual se respeta. Las palabras clave se convierten en
/// etiquetas. Devuelve si se aplicó.
pub(crate) fn apply(conn: &Connection, id: i64, meta: &FileMeta) -> Result<bool> {
    let applied = conn.execute(
        "UPDATE photos SET
             rating        = CASE WHEN rating_source = 'manual' OR ?2 IS NULL THEN rating
                                  ELSE ?2 END,
             rating_source = CASE WHEN rating_source = 'manual' THEN 'manual'
                                  WHEN ?2 IS NULL THEN rating_source
                                  ELSE 'archivo' END,
             description   = coalesce(description, ?3),
             camera        = coalesce(camera, ?4),
             lens          = coalesce(lens, ?5),
             metadata_at   = unixepoch()
         WHERE id = ?1 AND metadata_at IS NULL",
        params![id, meta.rating, meta.description, meta.camera, meta.lens],
    )? > 0;
    if !applied || meta.keywords.is_empty() {
        return Ok(applied);
    }
    let mut tagged = false;
    for keyword in &meta.keywords {
        let Ok(name) = normalize_tag_name(keyword) else {
            continue;
        };
        let tag = match find_by_name(conn, &name)? {
            Some(tag) => tag,
            None => {
                conn.execute("INSERT INTO tags (name) VALUES (?1)", [&name])?;
                conn.last_insert_rowid()
            }
        };
        tagged |= conn.execute(
            "INSERT OR IGNORE INTO photo_tags (photo_id, tag_id) VALUES (?1, ?2)",
            params![id, tag],
        )? > 0;
    }
    if tagged {
        refresh_search(conn, &[id])?;
    }
    Ok(true)
}

/// Fotos de la base que aún no tienen los metadatos del archivo.
pub(crate) fn pending(conn: &Connection) -> Result<Vec<(i64, String)>> {
    let mut stmt =
        conn.prepare("SELECT id, rel_path FROM photos WHERE metadata_at IS NULL ORDER BY id")?;
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

/// Estrellas que tenía una foto antes de cambiarlas, para deshacer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviousRating {
    pub id: i64,
    pub rating: u8,
    /// `archivo`, `manual` o `None`, como estaba.
    pub source: Option<String>,
}

impl Library {
    /// Pone `rating` estrellas (0 las quita) a las fotos `ids`, a mano: ya no
    /// se pisan al reindexar. Las de la Papelera y los ids que no existen se
    /// ignoran. Devuelve cómo estaban las que cambiaron, para deshacer con
    /// [`Library::restore_ratings`].
    pub fn set_rating(&mut self, ids: &[i64], rating: u8) -> Result<Vec<PreviousRating>> {
        if rating > 5 {
            return Err(Error::InvalidRating(rating));
        }
        let tx = self.conn.transaction()?;
        let mut previous = Vec::new();
        {
            let mut read = tx.prepare(
                "SELECT rating, rating_source FROM photos WHERE id = ?1 AND trashed_at IS NULL",
            )?;
            let mut write = tx
                .prepare("UPDATE photos SET rating = ?2, rating_source = 'manual' WHERE id = ?1")?;
            for &id in ids {
                let Some((old, source)) = read
                    .query_row([id], |r| {
                        Ok((r.get::<_, u8>(0)?, r.get::<_, Option<String>>(1)?))
                    })
                    .optional()?
                else {
                    continue;
                };
                if old == rating && source.as_deref() == Some("manual") {
                    continue;
                }
                write.execute(params![id, rating])?;
                previous.push(PreviousRating {
                    id,
                    rating: old,
                    source,
                });
            }
        }
        tx.commit()?;
        Ok(previous)
    }

    /// Deshace [`Library::set_rating`]: devuelve a cada foto las estrellas y
    /// el origen que tenía.
    pub fn restore_ratings(&mut self, previous: &[PreviousRating]) -> Result<()> {
        if let Some(bad) = previous.iter().find(|p| p.rating > 5) {
            return Err(Error::InvalidRating(bad.rating));
        }
        let tx = self.conn.transaction()?;
        {
            let mut stmt =
                tx.prepare("UPDATE photos SET rating = ?2, rating_source = ?3 WHERE id = ?1")?;
            for p in previous {
                stmt.execute(params![p.id, p.rating, p.source])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Lee el XMP, la valoración EXIF y la cámara de las fotos que todavía no
    /// los tienen, por ejemplo las importadas antes de la spec 024. Las fotos
    /// que no están en el disco se dejan para otra vez. Devuelve cuántas se
    /// leyeron. Solo lee los archivos.
    pub fn read_pending_metadata(&mut self) -> Result<u64> {
        let pending = pending(&self.conn)?;
        let mut done = 0;
        // Por lotes, para no tener la base bloqueada mucho rato.
        for chunk in pending.chunks(200) {
            let tx = self.conn.transaction()?;
            for (id, rel) in chunk {
                let path = RelPath::new(rel)?.to_path(&self.root);
                if !path.is_file() {
                    continue;
                }
                if apply(&tx, *id, &FileMeta::of_file(&path))? {
                    done += 1;
                }
            }
            tx.commit()?;
        }
        Ok(done)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::fs;

    use super::*;
    use crate::tags::tests::found;

    pub(crate) const NS: &str = r#"xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmlns:dc="http://purl.org/dc/elements/1.1/""#;

    /// Paquete XMP con valoración, descripción y palabras clave.
    pub(crate) fn xmp(rating: u8, description: &str, keywords: &[&str]) -> String {
        let items: String = keywords
            .iter()
            .map(|k| format!("<rdf:li>{k}</rdf:li>"))
            .collect();
        format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF {NS}><rdf:Description xmp:Rating="{rating}"><dc:description><rdf:Alt><rdf:li xml:lang="x-default">{description}</rdf:li></rdf:Alt></dc:description><dc:subject><rdf:Bag>{items}</rdf:Bag></dc:subject></rdf:Description></rdf:RDF></x:xmpmeta>"#
        )
    }

    /// PNG mínimo (cabecera) con un XMP en un fragmento `iTXt`.
    pub(crate) fn write_png_with_xmp(path: &Path, salt: &[u8], packet: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut chunk = |kind: &[u8], data: &[u8]| {
            v.extend((data.len() as u32).to_be_bytes());
            v.extend(kind);
            v.extend(data);
            v.extend([0, 0, 0, 0]);
        };
        let mut ihdr = 4u32.to_be_bytes().to_vec();
        ihdr.extend(3u32.to_be_bytes());
        ihdr.extend([8, 2, 0, 0, 0]);
        chunk(b"IHDR", &ihdr);
        let mut itxt = b"XML:com.adobe.xmp\0\0\0\0\0".to_vec();
        itxt.extend(packet.as_bytes());
        chunk(b"iTXt", &itxt);
        chunk(b"tEXt", salt);
        chunk(b"IEND", &[]);
        fs::write(path, v).unwrap();
    }

    fn row(lib: &Library, id: i64) -> (i64, Option<String>, Option<String>, bool) {
        lib.conn
            .query_row(
                "SELECT rating, rating_source, description, metadata_at IS NOT NULL
                 FROM photos WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap()
    }

    fn tag_names(lib: &Library, id: i64) -> Vec<String> {
        lib.photo_tags(id)
            .unwrap()
            .into_iter()
            .map(|t| t.name)
            .collect()
    }

    fn meta(rating: Option<u8>, keywords: &[&str]) -> FileMeta {
        FileMeta {
            rating,
            description: Some("Del XMP".into()),
            keywords: keywords.iter().map(|k| k.to_string()).collect(),
            camera: Some("Canon EOS 80D".into()),
            lens: None,
        }
    }

    #[test]
    fn aplica_una_sola_vez() {
        let (_dir, mut lib) = crate::tags::tests::library(1);
        assert!(apply(&lib.conn, 1, &meta(Some(4), &["Playa"])).unwrap());
        assert_eq!(
            row(&lib, 1),
            (4, Some("archivo".into()), Some("Del XMP".into()), true)
        );
        assert_eq!(tag_names(&lib, 1), ["Playa"]);
        assert_eq!(found(&lib, "playa"), [1]);

        // El usuario quita la etiqueta y pone 2 estrellas: una segunda lectura
        // no cambia nada.
        let tag = lib.tags().unwrap()[0].id;
        lib.remove_tag(&[1], tag).unwrap();
        lib.conn
            .execute(
                "UPDATE photos SET rating = 2, rating_source = 'manual' WHERE id = 1",
                [],
            )
            .unwrap();
        assert!(!apply(&lib.conn, 1, &meta(Some(5), &["Playa"])).unwrap());
        assert_eq!(row(&lib, 1).0, 2);
        assert!(tag_names(&lib, 1).is_empty());
    }

    #[test]
    fn respeta_la_valoracion_manual_y_la_descripcion() {
        let (_dir, lib) = crate::tags::tests::library(1);
        lib.conn
            .execute(
                "UPDATE photos SET rating = 1, rating_source = 'manual',
                                   description = 'De Takeout' WHERE id = 1",
                [],
            )
            .unwrap();
        apply(&lib.conn, 1, &meta(Some(5), &[])).unwrap();
        assert_eq!(
            row(&lib, 1),
            (1, Some("manual".into()), Some("De Takeout".into()), true)
        );
    }

    #[test]
    fn sin_valoracion_no_marca_el_origen() {
        let (_dir, lib) = crate::tags::tests::library(1);
        apply(&lib.conn, 1, &meta(None, &[])).unwrap();
        assert_eq!(row(&lib, 1).0, 0);
        assert_eq!(row(&lib, 1).1, None);
        assert!(row(&lib, 1).3);
    }

    #[test]
    fn reutiliza_etiquetas_y_descarta_nombres_invalidos() {
        let (_dir, mut lib) = crate::tags::tests::library(1);
        lib.create_tag("Montaña").unwrap();
        apply(&lib.conn, 1, &meta(None, &["montaña", "\u{7}", "Gatos"])).unwrap();
        let mut names = tag_names(&lib, 1);
        names.sort();
        assert_eq!(names, ["Gatos", "Montaña"]);
        assert_eq!(lib.tags().unwrap().len(), 2);
    }

    #[test]
    fn lee_el_xmp_de_un_archivo() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.png");
        write_png_with_xmp(&p, b"a", &xmp(3, "Hola", &["Uno", "Dos"]));
        let m = FileMeta::of_file(&p);
        assert_eq!(m.rating, Some(3));
        assert_eq!(m.description.as_deref(), Some("Hola"));
        assert_eq!(m.keywords, ["Uno", "Dos"]);
        assert_eq!(m.camera, None);
    }

    fn ratings(lib: &Library) -> Vec<(String, i64, Option<String>)> {
        let mut stmt = lib
            .conn
            .prepare("SELECT rel_path, rating, rating_source FROM photos ORDER BY rel_path")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn importar_lee_estrellas_y_palabras_clave() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(&dir.path().join("Biblioteca")).unwrap();
        let src = dir.path().join("Camara");
        write_png_with_xmp(
            &src.join("a.png"),
            b"a",
            &xmp(4, "Subida", &["Picos", "Montaña"]),
        );
        crate::index::tests::write_png(&src.join("b.png"), 4, 3, b"b");
        lib.import_from(&src).unwrap();

        let r = ratings(&lib);
        assert_eq!(r.len(), 2);
        let a = r.iter().find(|x| x.0.ends_with("a.png")).unwrap();
        assert_eq!((a.1, a.2.as_deref()), (4, Some("archivo")));
        let b = r.iter().find(|x| x.0.ends_with("b.png")).unwrap();
        assert_eq!((b.1, b.2.as_deref()), (0, None));
        assert_eq!(found(&lib, "picos").len(), 1);
        let mut names: Vec<String> = lib.tags().unwrap().into_iter().map(|t| t.name).collect();
        names.sort();
        assert_eq!(names, ["Montaña", "Picos"]);
        // Todas leídas: no queda nada pendiente.
        assert!(pending(&lib.conn).unwrap().is_empty());
    }

    #[test]
    fn reindexar_rellena_una_biblioteca_antigua_una_sola_vez() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(&dir.path().join("Biblioteca")).unwrap();
        let src = dir.path().join("Camara");
        write_png_with_xmp(&src.join("a.png"), b"a", &xmp(5, "", &["Playa"]));
        lib.import_from(&src).unwrap();
        // Como si se hubiera importado antes de la spec 024.
        lib.conn
            .execute_batch(
                "UPDATE photos SET rating = 0, rating_source = NULL, metadata_at = NULL;
                 DELETE FROM photo_tags; DELETE FROM tags;
                 UPDATE photo_search SET tags = '';",
            )
            .unwrap();
        assert_eq!(found(&lib, "playa"), Vec::<i64>::new());

        lib.index().unwrap();
        assert_eq!(ratings(&lib)[0].1, 5);
        assert_eq!(found(&lib, "playa").len(), 1);

        // El usuario cambia las estrellas y quita la etiqueta; reindexar no
        // lo deshace.
        let tag = lib.tags().unwrap()[0].id;
        let id = found(&lib, "playa")[0];
        lib.remove_tag(&[id], tag).unwrap();
        lib.conn
            .execute("UPDATE photos SET rating = 2, rating_source = 'manual'", [])
            .unwrap();
        lib.index().unwrap();
        assert_eq!(ratings(&lib)[0].1, 2);
        assert!(lib.photo_tags(id).unwrap().is_empty());
    }

    #[test]
    fn las_fotos_que_faltan_quedan_pendientes() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(&dir.path().join("Biblioteca")).unwrap();
        let src = dir.path().join("Camara");
        write_png_with_xmp(&src.join("a.png"), b"a", &xmp(3, "", &[]));
        lib.import_from(&src).unwrap();
        let rel: String = lib
            .conn
            .query_row("SELECT rel_path FROM photos", [], |r| r.get(0))
            .unwrap();
        lib.conn
            .execute("UPDATE photos SET metadata_at = NULL", [])
            .unwrap();
        let path = lib.root.join(&rel);
        let saved = dir.path().join("fuera.png");
        fs::rename(&path, &saved).unwrap();
        assert_eq!(lib.read_pending_metadata().unwrap(), 0);
        assert_eq!(pending(&lib.conn).unwrap().len(), 1);
        fs::rename(&saved, &path).unwrap();
        assert_eq!(lib.read_pending_metadata().unwrap(), 1);
    }

    #[test]
    fn el_xmp_suelto_gana() {
        let file = meta(Some(2), &["Playa"]);
        let sidecar = XmpData {
            rating: Some(5),
            description: None,
            keywords: vec!["playa".into(), "Verano".into()],
        };
        let merged = file.with_sidecar(sidecar);
        assert_eq!(merged.rating, Some(5));
        assert_eq!(merged.description.as_deref(), Some("Del XMP"));
        assert_eq!(merged.keywords, ["playa", "Verano"]);
    }

    fn rating_of(lib: &Library, name: &str) -> i64 {
        ratings(lib)
            .into_iter()
            .find(|r| r.0.ends_with(name))
            .unwrap_or_else(|| panic!("falta {name}"))
            .1
    }

    #[test]
    fn importar_usa_el_xmp_junto_al_origen() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(&dir.path().join("Biblioteca")).unwrap();
        let src = dir.path().join("Camara");
        // Incrustado 2, al lado 5: gana el de al lado.
        write_png_with_xmp(&src.join("a.png"), b"a", &xmp(2, "", &["Dentro"]));
        fs::write(src.join("a.png.xmp"), xmp(5, "", &["Fuera"])).unwrap();
        // foto.xmp de Lightroom para el RAW y el JPEG, salvo el que tiene el suyo.
        crate::index::tests::write_png(&src.join("b.png"), 4, 3, b"b");
        crate::index::tests::write_png(&src.join("b.webp"), 4, 3, b"c");
        fs::write(src.join("b.xmp"), xmp(3, "", &[])).unwrap();
        crate::index::tests::write_png(&src.join("c.png"), 4, 3, b"d");
        fs::write(src.join("c.xmp"), xmp(1, "", &[])).unwrap();
        fs::write(src.join("c.png.XMP"), xmp(4, "", &[])).unwrap();
        // Un .xmp mal formado se ignora.
        write_png_with_xmp(&src.join("d.png"), b"e", &xmp(2, "", &[]));
        fs::write(src.join("d.png.xmp"), "<x:xmpmeta><rdf:RDF>").unwrap();
        let report = lib.import_from(&src).unwrap();
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        assert_eq!(rating_of(&lib, "a.png"), 5);
        assert_eq!(rating_of(&lib, "/b.png"), 3);
        assert_eq!(rating_of(&lib, "/b.webp"), 3);
        assert_eq!(rating_of(&lib, "c.png"), 4);
        assert_eq!(rating_of(&lib, "d.png"), 2);
        let mut names: Vec<String> = lib.tags().unwrap().into_iter().map(|t| t.name).collect();
        names.sort();
        assert_eq!(names, ["Dentro", "Fuera"]);
        // Los .xmp no se copian a la biblioteca.
        let copied: Vec<_> = walkdir::WalkDir::new(lib.root.join(crate::library::ORIGINALS_DIR))
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("xmp"))
            })
            .collect();
        assert!(copied.is_empty());
    }

    fn estrellas(lib: &Library) -> Vec<(i64, Option<String>)> {
        let mut stmt = lib
            .conn
            .prepare("SELECT rating, rating_source FROM photos ORDER BY id")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn valorar_y_deshacer() {
        let (_dir, mut lib) = crate::tags::tests::library(4);
        lib.conn
            .execute_batch(
                "UPDATE photos SET rating = 4, rating_source = 'archivo' WHERE id = 1;
                 UPDATE photos SET rating = 3, rating_source = 'manual' WHERE id = 2;
                 UPDATE photos SET trashed_at = 1 WHERE id = 4;",
            )
            .unwrap();
        let antes = estrellas(&lib);
        let prev = lib.set_rating(&[1, 2, 3, 4, 99], 3).unwrap();
        // La 2 ya tenía 3 a mano; la 4 está en la Papelera; la 99 no existe.
        assert_eq!(prev.iter().map(|p| p.id).collect::<Vec<_>>(), [1, 3]);
        assert_eq!(
            estrellas(&lib)[..3],
            [
                (3, Some("manual".into())),
                (3, Some("manual".into())),
                (3, Some("manual".into()))
            ]
        );
        lib.restore_ratings(&prev).unwrap();
        assert_eq!(estrellas(&lib), antes);

        // 0 quita las estrellas, y sigue siendo manual: reindexar no las pone.
        lib.set_rating(&[1], 0).unwrap();
        assert_eq!(estrellas(&lib)[0], (0, Some("manual".into())));
        assert!(matches!(
            lib.set_rating(&[1], 6),
            Err(Error::InvalidRating(6))
        ));
    }

    #[test]
    fn la_cuadricula_y_el_visor_ven_las_estrellas() {
        let (_dir, mut lib) = crate::tags::tests::library(2);
        lib.set_rating(&[2], 5).unwrap();
        let page = lib.timeline_page(None, 10).unwrap();
        let r: Vec<(i64, u8)> = page.photos.iter().map(|p| (p.id, p.rating)).collect();
        assert_eq!(r, [(2, 5), (1, 0)]);
        assert_eq!(lib.photo_details(2).unwrap().unwrap().rating, 5);
    }
}
