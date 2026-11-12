//! Recetas de edición en disco (spec 009).
//!
//! Cada foto editada tiene una receta JSON en
//! `.mnemos/ediciones/<ab>/<huella>.json`, con la misma huella y el mismo
//! reparto que las miniaturas: la edición sigue a la foto aunque se mueva o se
//! renombre, y dos copias idénticas la comparten. La receta guarda todas las
//! versiones; guardar añade una y volver al original solo cambia cuál se
//! muestra. El original nunca se toca.
//!
//! Se escribe primero en `.mnemos/tmp/` y luego se renombra sobre la receta,
//! así que un corte a mitad deja la anterior intacta.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use mnemos_imaging::recipe::{Geometry, OriginalRef, Recipe, RecipeError};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::files::{hash_file, unix_secs};
use crate::library::{DATA_DIR, Library, TMP_DIR};
use crate::paths::RelPath;

/// Carpeta de las recetas, relativa a la raíz.
pub const EDITS_DIR: &str = ".mnemos/ediciones";

/// Sufijo con el que se aparta una receta dañada al guardar otra encima.
const DAMAGED_SUFFIX: &str = ".danado";

/// Ruta relativa de la receta de las fotos con esta huella.
pub fn recipe_rel_path(content_hash: &str) -> String {
    let prefix = content_hash.get(..2).unwrap_or("00");
    format!("{EDITS_DIR}/{prefix}/{content_hash}.json")
}

/// Lo que hay en disco para una foto.
#[derive(Debug, Clone, PartialEq)]
pub enum StoredRecipe {
    /// La foto no se ha editado nunca.
    None,
    Valid(Recipe),
    /// Hay un archivo, pero no se puede usar: la foto se muestra sin editar.
    Damaged(String),
}

impl StoredRecipe {
    /// Geometría que hay que aplicar al mostrar la foto: la de la versión
    /// actual, o ninguna si se muestra el original o la receta está dañada.
    pub fn geometry(&self) -> Geometry {
        match self {
            Self::Valid(recipe) => recipe.current_geometry(),
            Self::None | Self::Damaged(_) => Geometry::default(),
        }
    }
}

/// Resumen de las ediciones de una foto para la sección «Ediciones» del
/// visor.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EditSummary {
    /// Versión que se muestra, o `None` si se muestra el original.
    pub current: Option<u32>,
    /// De la más antigua a la más nueva.
    pub versions: Vec<EditVersion>,
    /// Por qué no se puede usar la receta, si está dañada.
    pub damaged: Option<String>,
    /// `false` si la receta es de una versión más nueva de MNEMOS y no se
    /// puede guardar encima.
    pub writable: bool,
}

/// Una versión de la receta, tal como se lista en el visor.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EditVersion {
    pub id: u32,
    pub created_at: i64,
    pub rotate: u16,
    pub flip_horizontal: bool,
    pub cropped: bool,
    /// Recorte de la versión, para abrir el editor sobre ella.
    pub crop: Option<mnemos_imaging::recipe::Crop>,
}

impl From<&StoredRecipe> for EditSummary {
    fn from(stored: &StoredRecipe) -> Self {
        match stored {
            StoredRecipe::None => Self {
                current: None,
                versions: Vec::new(),
                damaged: None,
                writable: true,
            },
            StoredRecipe::Damaged(reason) => Self {
                current: None,
                versions: Vec::new(),
                damaged: Some(reason.clone()),
                writable: true,
            },
            StoredRecipe::Valid(recipe) => Self {
                current: recipe.current,
                versions: recipe
                    .versions
                    .iter()
                    .map(|v| EditVersion {
                        id: v.id,
                        created_at: v.created_at,
                        rotate: v.geometry.rotate,
                        flip_horizontal: v.geometry.flip_horizontal,
                        cropped: v.geometry.crop.is_some(),
                        crop: v.geometry.crop,
                    })
                    .collect(),
                damaged: None,
                writable: recipe.is_writable(),
            },
        }
    }
}

/// Lo que hace falta de una foto para leer o guardar su receta.
struct EditTarget {
    rel_path: RelPath,
    content_hash: Option<String>,
    orientation: u16,
    width: Option<u32>,
    height: Option<u32>,
}

impl Library {
    /// Receta de la foto `id`. Una receta dañada o que no es de este archivo
    /// no es un error: se devuelve como [`StoredRecipe::Damaged`] para que la
    /// foto se siga viendo, sin editar.
    pub fn photo_recipe(&self, id: i64) -> Result<StoredRecipe> {
        let target = self.edit_target(id)?;
        match &target.content_hash {
            Some(hash) => Ok(read_recipe(&self.root, hash)?),
            None => Ok(StoredRecipe::None),
        }
    }

    /// Resumen de las ediciones de la foto `id`.
    pub fn edit_summary(&self, id: i64) -> Result<EditSummary> {
        Ok(EditSummary::from(&self.photo_recipe(id)?))
    }

    /// Guarda una versión nueva con `geometry`, la hace actual y devuelve su
    /// id («Guardar como original»). El archivo original no se toca.
    ///
    /// Si la foto aún no tiene huella, se calcula antes. Si la receta que había
    /// está dañada, se aparta como `<huella>.json.danado` y se empieza otra.
    /// Si es de un formato más nuevo, no se guarda nada.
    pub fn save_edit(&mut self, id: i64, geometry: Geometry) -> Result<u32> {
        let target = self.edit_target(id)?;
        let path = target.rel_path.to_path(&self.root);
        if !path.is_file() {
            return Err(Error::OriginalMissing(target.rel_path.as_str().to_owned()));
        }
        let hash = match &target.content_hash {
            Some(hash) => hash.clone(),
            None => {
                let hash = hash_file(&path)?;
                self.conn.execute(
                    "UPDATE photos SET content_hash = ?2 WHERE id = ?1",
                    rusqlite::params![id, hash],
                )?;
                hash
            }
        };
        let mut recipe = match read_recipe(&self.root, &hash)? {
            StoredRecipe::Valid(recipe) => recipe,
            StoredRecipe::None => Recipe::new(original_ref(&target, &path, &hash)),
            StoredRecipe::Damaged(_) => {
                set_aside(&self.root, &hash)?;
                Recipe::new(original_ref(&target, &path, &hash))
            }
        };
        let version = recipe.add_version(geometry, unix_secs(SystemTime::now()))?;
        write_recipe(&self.root, &hash, &recipe)?;
        set_cached_version(&self.conn, &hash, Some(version))?;
        remove_stale_thumbnails(&self.root, &hash, Some(version));
        Ok(version)
    }

    /// Muestra la versión `version` de la foto `id`, o el original si es
    /// `None` («Volver al original»). No borra ninguna versión.
    pub fn set_edit_version(&mut self, id: i64, version: Option<u32>) -> Result<()> {
        let target = self.edit_target(id)?;
        let recipe = target
            .content_hash
            .as_deref()
            .map(|hash| read_recipe(&self.root, hash))
            .transpose()?;
        match (recipe, target.content_hash) {
            (Some(StoredRecipe::Valid(mut recipe)), Some(hash)) => {
                if !recipe.is_writable() {
                    return Err(RecipeError::Invalid(format!(
                        "la receta es del formato {}, más nuevo que el que entiende esta versión",
                        recipe.format
                    ))
                    .into());
                }
                recipe.set_current(version)?;
                write_recipe(&self.root, &hash, &recipe)?;
                set_cached_version(&self.conn, &hash, version)?;
                remove_stale_thumbnails(&self.root, &hash, version);
                Ok(())
            }
            // Sin receta (o dañada) ya se ve el original.
            _ if version.is_none() => Ok(()),
            _ => Err(RecipeError::Invalid(format!(
                "la foto {id} no tiene la versión {}",
                version.unwrap_or_default()
            ))
            .into()),
        }
    }

    /// Vuelve a calcular `photos.edit_version` a partir de las recetas que
    /// hay en `.mnemos/ediciones/`. Lo llama el indexado, para que una
    /// biblioteca copiada o con la base de datos regenerada recupere sus
    /// ediciones. Solo recorre la carpeta de recetas, no lee cada foto.
    pub(crate) fn sync_edit_versions(conn: &Connection, root: &Path) -> Result<()> {
        let mut versions = Vec::new();
        let dir = root.join(EDITS_DIR);
        for prefix in read_dir_or_empty(&dir)? {
            for entry in read_dir_or_empty(&prefix)? {
                let Some(hash) = entry
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|n| n.strip_suffix(".json"))
                    .filter(|h| !h.is_empty() && h.bytes().all(|b| b.is_ascii_hexdigit()))
                    .map(str::to_owned)
                else {
                    continue;
                };
                if let StoredRecipe::Valid(recipe) = read_recipe(root, &hash)? {
                    versions.push((hash, recipe.current));
                }
            }
        }
        conn.execute(
            "UPDATE photos SET edit_version = NULL WHERE edit_version IS NOT NULL",
            [],
        )?;
        for (hash, version) in versions {
            set_cached_version(conn, &hash, version)?;
        }
        Ok(())
    }

    fn edit_target(&self, id: i64) -> Result<EditTarget> {
        self.conn
            .query_row(
                "SELECT rel_path, content_hash, coalesce(orientation, 1), width, height
                 FROM photos WHERE id = ?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, u16>(2)?,
                        r.get::<_, Option<u32>>(3)?,
                        r.get::<_, Option<u32>>(4)?,
                    ))
                },
            )
            .optional()?
            .map(|(rel_path, content_hash, orientation, width, height)| {
                Ok(EditTarget {
                    rel_path: RelPath::new(&rel_path)?,
                    content_hash,
                    orientation,
                    width,
                    height,
                })
            })
            .ok_or(Error::PhotoNotFound(id))?
    }
}

/// Apunta en la base de datos qué versión se ve en las fotos con esta huella.
fn set_cached_version(conn: &Connection, hash: &str, version: Option<u32>) -> Result<()> {
    conn.execute(
        "UPDATE photos SET edit_version = ?2 WHERE content_hash = ?1",
        rusqlite::params![hash, version],
    )?;
    Ok(())
}

/// Borra las miniaturas de versiones editadas de esta huella que ya no se ven
/// (`<huella>-e<n>.jpg` con `n` distinto de `keep`). Son una caché: si vuelven
/// a hacer falta, se regeneran. La del original no se toca nunca. Un fallo al
/// borrar no importa.
fn remove_stale_thumbnails(root: &Path, hash: &str, keep: Option<u32>) {
    let dir = root
        .join(crate::thumbnails::thumbnail_rel_path(hash))
        .with_file_name("");
    let Ok(entries) = read_dir_or_empty(&dir) else {
        return;
    };
    let prefix = format!("{hash}-e");
    for path in entries {
        let version = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix(&prefix))
            .and_then(|n| n.strip_suffix(".jpg"))
            .and_then(|n| n.parse::<u32>().ok());
        if version.is_some() && version != keep {
            let _ = fs::remove_file(path);
        }
    }
}

/// Entradas de una carpeta; ninguna si no existe.
fn read_dir_or_empty(dir: &Path) -> io::Result<Vec<PathBuf>> {
    match fs::read_dir(dir) {
        Ok(entries) => entries.map(|e| e.map(|e| e.path())).collect(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

/// Huella y tamaño ya orientado del original, para la receta.
fn original_ref(target: &EditTarget, path: &Path, hash: &str) -> OriginalRef {
    let (w, h) = match (target.width, target.height) {
        (Some(w), Some(h)) => (w, h),
        _ => mnemos_imaging::dimensions(path).unwrap_or((1, 1)),
    };
    let (width, height) = if mnemos_imaging::swaps_dimensions(target.orientation) {
        (h, w)
    } else {
        (w, h)
    };
    OriginalRef {
        hash: hash.to_owned(),
        width: width.max(1),
        height: height.max(1),
    }
}

/// Lee la receta de una huella. Solo un fallo de lectura del disco es un
/// error; un archivo que no se entiende es [`StoredRecipe::Damaged`].
pub(crate) fn read_recipe(root: &Path, hash: &str) -> io::Result<StoredRecipe> {
    let bytes = match fs::read(root.join(recipe_rel_path(hash))) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(StoredRecipe::None),
        Err(err) => return Err(err),
    };
    Ok(match Recipe::parse(&bytes) {
        Ok(recipe) if recipe.original.hash.eq_ignore_ascii_case(hash) => {
            StoredRecipe::Valid(recipe)
        }
        Ok(_) => StoredRecipe::Damaged("la receta es de otro archivo".into()),
        Err(err) => StoredRecipe::Damaged(err.to_string()),
    })
}

/// Escribe la receta en `.mnemos/tmp/` y la renombra sobre la anterior.
fn write_recipe(root: &Path, hash: &str, recipe: &Recipe) -> Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let json = recipe.to_json()?;
    let dest = root.join(recipe_rel_path(hash));
    let tmp_dir = root.join(DATA_DIR).join(TMP_DIR);
    fs::create_dir_all(&tmp_dir)?;
    let part = tmp_dir.join(format!(
        "{hash}-{}-{}.json.part",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let written = fs::write(&part, &json)
        .and_then(|()| fs::File::open(&part)?.sync_all())
        .and_then(|()| fs::create_dir_all(dest.parent().unwrap_or(root)))
        .and_then(|()| fs::rename(&part, &dest));
    if let Err(err) = written {
        let _ = fs::remove_file(&part);
        return Err(err.into());
    }
    Ok(())
}

/// Aparta una receta dañada con un nombre libre (`.json.danado`,
/// `.json.danado-2`…) en vez de borrarla.
fn set_aside(root: &Path, hash: &str) -> io::Result<PathBuf> {
    let src = root.join(recipe_rel_path(hash));
    let mut dest = src.with_extension(format!("json{DAMAGED_SUFFIX}"));
    let mut n = 2;
    while dest.exists() {
        dest = src.with_extension(format!("json{DAMAGED_SUFFIX}-{n}"));
        n += 1;
    }
    fs::rename(&src, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use mnemos_imaging::recipe::Crop;

    use super::*;
    use crate::library::ORIGINALS_DIR;

    struct Setup {
        dir: tempfile::TempDir,
        lib: Library,
        id: i64,
    }

    impl Setup {
        fn root(&self) -> &Path {
            self.dir.path()
        }

        fn hash(&self) -> String {
            self.lib
                .connection()
                .query_row(
                    "SELECT content_hash FROM photos WHERE id = ?1",
                    [self.id],
                    |r| r.get(0),
                )
                .unwrap()
        }

        fn recipe_path(&self) -> PathBuf {
            self.root().join(recipe_rel_path(&self.hash()))
        }
    }

    /// Biblioteca con una foto JPEG de 600 × 400 con un nombre con acentos.
    fn setup() -> Setup {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        let path = dir.path().join(ORIGINALS_DIR).join("Año ñ.jpg");
        image::RgbImage::from_fn(600, 400, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 90])
        })
        .save_with_format(&path, image::ImageFormat::Jpeg)
        .unwrap();
        lib.index().unwrap();
        let id = lib
            .connection()
            .query_row("SELECT id FROM photos", [], |r| r.get(0))
            .unwrap();
        Setup { dir, lib, id }
    }

    fn giro() -> Geometry {
        Geometry {
            rotate: 90,
            ..Geometry::default()
        }
    }

    fn recorte() -> Geometry {
        Geometry {
            crop: Some(Crop {
                x: 0.1,
                y: 0.1,
                width: 0.5,
                height: 0.5,
            }),
            ..giro()
        }
    }

    fn original(s: &Setup) -> (Vec<u8>, SystemTime) {
        let path = s.root().join(ORIGINALS_DIR).join("Año ñ.jpg");
        (
            fs::read(&path).unwrap(),
            fs::metadata(&path).unwrap().modified().unwrap(),
        )
    }

    #[test]
    fn sin_editar_no_hay_receta() {
        let s = setup();
        assert_eq!(s.lib.photo_recipe(s.id).unwrap(), StoredRecipe::None);
        let summary = s.lib.edit_summary(s.id).unwrap();
        assert!(summary.versions.is_empty() && summary.current.is_none());
        assert!(matches!(
            s.lib.photo_recipe(999),
            Err(Error::PhotoNotFound(999))
        ));
    }

    #[test]
    fn guarda_versiones_y_vuelve_al_original_sin_tocarlo() {
        let mut s = setup();
        let before = original(&s);

        assert_eq!(s.lib.save_edit(s.id, giro()).unwrap(), 1);
        assert_eq!(s.lib.save_edit(s.id, recorte()).unwrap(), 2);
        assert_eq!(s.lib.photo_recipe(s.id).unwrap().geometry(), recorte());
        assert!(s.recipe_path().is_file());

        // Volver al original no borra el historial.
        s.lib.set_edit_version(s.id, None).unwrap();
        assert!(s.lib.photo_recipe(s.id).unwrap().geometry().is_identity());
        let summary = s.lib.edit_summary(s.id).unwrap();
        assert_eq!(summary.current, None);
        assert_eq!(
            summary.versions.iter().map(|v| v.id).collect::<Vec<_>>(),
            [1, 2]
        );
        assert!(summary.versions[1].cropped && !summary.versions[0].cropped);
        assert_eq!(summary.versions[1].crop, recorte().crop);

        // Y se puede volver exactamente a la versión 2.
        s.lib.set_edit_version(s.id, Some(2)).unwrap();
        assert_eq!(s.lib.photo_recipe(s.id).unwrap().geometry(), recorte());
        assert!(s.lib.set_edit_version(s.id, Some(9)).is_err());
        assert_eq!(s.lib.edit_summary(s.id).unwrap().current, Some(2));

        assert_eq!(original(&s), before);
        // Nada a medias en tmp.
        let tmp = s.root().join(DATA_DIR).join(TMP_DIR);
        assert_eq!(fs::read_dir(tmp).map(|d| d.count()).unwrap_or(0), 0);
    }

    #[test]
    fn la_receta_guarda_el_tamano_ya_orientado() {
        let mut s = setup();
        s.lib
            .connection()
            .execute("UPDATE photos SET orientation = 6", [])
            .unwrap();
        s.lib.save_edit(s.id, giro()).unwrap();
        let StoredRecipe::Valid(recipe) = s.lib.photo_recipe(s.id).unwrap() else {
            panic!("receta no válida");
        };
        assert_eq!((recipe.original.width, recipe.original.height), (400, 600));
        assert_eq!(recipe.original.hash, s.hash());
    }

    #[test]
    fn un_fallo_al_escribir_deja_la_receta_anterior_intacta() {
        let mut s = setup();
        s.lib.save_edit(s.id, giro()).unwrap();
        let before = fs::read(s.recipe_path()).unwrap();

        // Sin poder crear `.mnemos/tmp/`, la escritura falla antes de tocar nada.
        let tmp = s.root().join(DATA_DIR).join(TMP_DIR);
        let _ = fs::remove_dir_all(&tmp);
        fs::write(&tmp, b"no es una carpeta").unwrap();
        assert!(s.lib.save_edit(s.id, recorte()).is_err());
        assert!(s.lib.set_edit_version(s.id, None).is_err());

        assert_eq!(fs::read(s.recipe_path()).unwrap(), before);
        assert_eq!(s.lib.photo_recipe(s.id).unwrap().geometry(), giro());
    }

    #[test]
    fn una_receta_danada_se_muestra_sin_editar_y_se_aparta_al_guardar() {
        let mut s = setup();
        s.lib.save_edit(s.id, giro()).unwrap();
        let path = s.recipe_path();
        fs::write(&path, b"{ roto").unwrap();

        let stored = s.lib.photo_recipe(s.id).unwrap();
        assert!(matches!(stored, StoredRecipe::Damaged(_)));
        assert!(stored.geometry().is_identity());
        assert!(s.lib.edit_summary(s.id).unwrap().damaged.is_some());

        // Guardar empieza una receta nueva y conserva la dañada.
        assert_eq!(s.lib.save_edit(s.id, recorte()).unwrap(), 1);
        let aparte = path.with_extension("json.danado");
        assert_eq!(fs::read(&aparte).unwrap(), b"{ roto");
        assert_eq!(s.lib.photo_recipe(s.id).unwrap().geometry(), recorte());

        // Una segunda vez no pisa la primera.
        fs::write(&path, b"[]").unwrap();
        s.lib.save_edit(s.id, giro()).unwrap();
        assert_eq!(fs::read(&aparte).unwrap(), b"{ roto");
        assert_eq!(
            fs::read(path.with_extension("json.danado-2")).unwrap(),
            b"[]"
        );
    }

    #[test]
    fn una_receta_de_otro_archivo_no_se_aplica() {
        let mut s = setup();
        s.lib.save_edit(s.id, giro()).unwrap();
        let path = s.recipe_path();
        let json = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        fs::write(&path, json.replace(&s.hash(), "abcdef")).unwrap();
        assert!(matches!(
            s.lib.photo_recipe(s.id).unwrap(),
            StoredRecipe::Damaged(_)
        ));
    }

    #[test]
    fn una_receta_de_un_formato_futuro_se_aplica_pero_no_se_pisa() {
        let mut s = setup();
        s.lib.save_edit(s.id, giro()).unwrap();
        let path = s.recipe_path();
        let json = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        let futura = json.replace("\"format\": 1", "\"format\": 2");
        fs::write(&path, &futura).unwrap();

        assert_eq!(s.lib.photo_recipe(s.id).unwrap().geometry(), giro());
        assert!(!s.lib.edit_summary(s.id).unwrap().writable);
        assert!(s.lib.save_edit(s.id, recorte()).is_err());
        assert!(s.lib.set_edit_version(s.id, None).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), futura);
    }

    #[test]
    fn la_receta_sigue_a_la_foto_al_moverla_y_volver_a_indexar() {
        let mut s = setup();
        s.lib.save_edit(s.id, recorte()).unwrap();
        let originales = s.root().join(ORIGINALS_DIR);
        fs::create_dir_all(originales.join("2024")).unwrap();
        fs::rename(
            originales.join("Año ñ.jpg"),
            originales.join("2024/movida.jpg"),
        )
        .unwrap();
        s.lib.index().unwrap();
        let id: i64 = s
            .lib
            .connection()
            .query_row("SELECT id FROM photos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(s.lib.photo_recipe(id).unwrap().geometry(), recorte());
    }

    #[test]
    fn una_foto_sin_huella_la_calcula_al_guardar() {
        let mut s = setup();
        let hash = s.hash();
        s.lib
            .connection()
            .execute("UPDATE photos SET content_hash = NULL", [])
            .unwrap();
        assert_eq!(s.lib.photo_recipe(s.id).unwrap(), StoredRecipe::None);
        s.lib.save_edit(s.id, giro()).unwrap();
        assert_eq!(s.hash(), hash);
        assert!(s.root().join(recipe_rel_path(&hash)).is_file());
    }

    fn cached(s: &Setup) -> Option<u32> {
        s.lib
            .connection()
            .query_row(
                "SELECT edit_version FROM photos WHERE id = ?1",
                [s.id],
                |r| r.get(0),
            )
            .unwrap()
    }

    #[test]
    fn la_base_de_datos_apunta_la_version_que_se_ve() {
        let mut s = setup();
        assert_eq!(cached(&s), None);
        s.lib.save_edit(s.id, giro()).unwrap();
        s.lib.save_edit(s.id, recorte()).unwrap();
        assert_eq!(cached(&s), Some(2));
        s.lib.set_edit_version(s.id, Some(1)).unwrap();
        assert_eq!(cached(&s), Some(1));
        s.lib.set_edit_version(s.id, None).unwrap();
        assert_eq!(cached(&s), None);
    }

    #[test]
    fn al_indexar_se_recupera_la_version_desde_las_recetas() {
        let mut s = setup();
        s.lib.save_edit(s.id, giro()).unwrap();
        s.lib.save_edit(s.id, recorte()).unwrap();
        // Base de datos desfasada: el indexado la corrige.
        s.lib
            .connection()
            .execute("UPDATE photos SET edit_version = NULL", [])
            .unwrap();
        s.lib.index().unwrap();
        assert_eq!(cached(&s), Some(2));

        // Una receta dañada no cuenta.
        fs::write(s.recipe_path(), b"{ roto").unwrap();
        s.lib.index().unwrap();
        assert_eq!(cached(&s), None);
    }

    #[test]
    fn una_biblioteca_copiada_o_sin_base_de_datos_conserva_sus_ediciones() {
        let mut s = setup();
        s.lib.save_edit(s.id, recorte()).unwrap();
        let before = original(&s);

        // Copia de la biblioteca a otra ruta, sin la base de datos.
        let copia = tempfile::tempdir().unwrap();
        copy_dir(s.root(), copia.path());
        fs::remove_file(copia.path().join(DATA_DIR).join("mnemos.db")).unwrap();
        let mut lib = Library::create(copia.path()).unwrap();
        lib.index().unwrap();
        let id: i64 = lib
            .connection()
            .query_row("SELECT id FROM photos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(lib.photo_recipe(id).unwrap().geometry(), recorte());
        let version: Option<u32> = lib
            .connection()
            .query_row("SELECT edit_version FROM photos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, Some(1));
        assert_eq!(original(&s), before);
    }

    fn copy_dir(src: &Path, dest: &Path) {
        for entry in walkdir::WalkDir::new(src) {
            let entry = entry.unwrap();
            let target = dest.join(entry.path().strip_prefix(src).unwrap());
            if entry.file_type().is_dir() {
                fs::create_dir_all(&target).unwrap();
            } else {
                fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    #[test]
    fn la_miniatura_editada_se_genera_aparte_y_las_viejas_se_borran() {
        use crate::thumbnails::{thumbnail_rel_path, thumbnail_rel_path_for};
        let mut s = setup();
        s.lib.generate_thumbnails().unwrap();
        let hash = s.hash();
        let original_thumb = s.root().join(thumbnail_rel_path(&hash));
        let (w, h) = image::image_dimensions(&original_thumb).unwrap();
        assert_eq!((w, h), (540, 360));

        // Girada: la miniatura de la versión 1 sale vertical.
        assert_eq!(s.lib.save_edit(s.id, giro()).unwrap(), 1);
        let jobs = s.lib.pending_thumbnails().unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].edit_version, Some(1));
        s.lib.generate_thumbnails().unwrap();
        let v1 = s.root().join(thumbnail_rel_path_for(&hash, Some(1)));
        assert_eq!(image::image_dimensions(&v1).unwrap(), (240, 360));
        assert!(s.lib.pending_thumbnails().unwrap().is_empty());

        // Al guardar la versión 2, la de la 1 sobra; la del original no.
        s.lib.save_edit(s.id, recorte()).unwrap();
        assert!(!v1.exists());
        s.lib.generate_thumbnails().unwrap();
        let v2 = s.root().join(thumbnail_rel_path_for(&hash, Some(2)));
        assert!(v2.is_file());

        // Volver al original borra la editada y usa la de siempre.
        s.lib.set_edit_version(s.id, None).unwrap();
        assert!(!v2.exists());
        assert!(original_thumb.is_file());
        assert!(s.lib.pending_thumbnails().unwrap().is_empty());
    }

    #[test]
    fn una_miniatura_editada_que_falla_no_bloquea_la_del_original() {
        let mut s = setup();
        s.lib.save_edit(s.id, giro()).unwrap();
        fs::write(s.recipe_path(), b"{ roto").unwrap();
        // La base de datos aún cree que se ve la versión 1.
        let report = s.lib.generate_thumbnails().unwrap();
        assert_eq!(report.failed.len(), 1);
        let fallos: i64 = s
            .lib
            .connection()
            .query_row("SELECT count(*) FROM thumbnail_failures", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fallos, 0);
        // Al indexar se corrige y sale la del original.
        s.lib.index().unwrap();
        let jobs = s.lib.pending_thumbnails().unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].edit_version, None);
    }

    #[test]
    fn los_datos_para_mostrar_llevan_la_edicion_que_se_ve() {
        let mut s = setup();
        assert_eq!(s.lib.photo_media(s.id).unwrap().unwrap().edit, None);
        s.lib.save_edit(s.id, giro()).unwrap();
        s.lib.save_edit(s.id, recorte()).unwrap();
        let edit = s.lib.photo_media(s.id).unwrap().unwrap().edit.unwrap();
        assert_eq!((edit.version, edit.geometry), (2, recorte()));

        s.lib.set_edit_version(s.id, Some(1)).unwrap();
        let edit = s.lib.photo_media(s.id).unwrap().unwrap().edit.unwrap();
        assert_eq!((edit.version, edit.geometry), (1, giro()));

        s.lib.set_edit_version(s.id, None).unwrap();
        assert_eq!(s.lib.photo_media(s.id).unwrap().unwrap().edit, None);

        // Una receta dañada muestra el original aunque la caché diga otra cosa.
        s.lib.set_edit_version(s.id, Some(2)).unwrap();
        fs::write(s.recipe_path(), b"{ roto").unwrap();
        assert_eq!(s.lib.photo_media(s.id).unwrap().unwrap().edit, None);
    }

    #[test]
    fn tras_guardar_falta_solo_la_miniatura_de_esa_foto() {
        let mut s = setup();
        s.lib.generate_thumbnails().unwrap();
        assert_eq!(s.lib.pending_thumbnail(s.id).unwrap(), None);
        s.lib.save_edit(s.id, giro()).unwrap();
        let job = s.lib.pending_thumbnail(s.id).unwrap().unwrap();
        assert_eq!((job.photo_id, job.edit_version), (s.id, Some(1)));
        assert_eq!(s.lib.pending_thumbnail(s.id + 1).unwrap(), None);
    }

    #[test]
    fn no_se_edita_una_foto_que_no_esta_en_el_disco() {
        let mut s = setup();
        fs::remove_file(s.root().join(ORIGINALS_DIR).join("Año ñ.jpg")).unwrap();
        assert!(matches!(
            s.lib.save_edit(s.id, giro()),
            Err(Error::OriginalMissing(_))
        ));
    }
}
