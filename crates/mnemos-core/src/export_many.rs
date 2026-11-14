//! Exportar varias fotos a la vez (spec 017).
//!
//! Cada foto se exporta con [`crate::export::export`], igual que desde el
//! visor (con su edición, si la tiene). Una foto con un problema propio
//! (falta su archivo, no se puede reducir…) se salta y se sigue; un error al
//! escribir en el destino, o la biblioteca desconectada, detiene el lote.
//! Nunca toca los originales ni la base de datos.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{Error, Result};
use crate::export::{ExportSize, export, inside};
use crate::library::{Library, ORIGINALS_DIR, PhotoMedia};
use crate::timeline::DATE;

/// Una foto del lote: su nombre, para el resumen, y lo que hace falta para
/// exportarla sin la base de datos.
#[derive(Debug, Clone)]
pub struct ExportItem {
    pub id: i64,
    pub name: String,
    pub media: PhotoMedia,
}

/// Una foto que no se pudo exportar y por qué, en lenguaje claro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub name: String,
    pub reason: String,
}

/// Cómo acabó el lote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchEnd {
    /// Se intentaron todas las fotos.
    Finished,
    /// Se pidió cancelar: se paró tras la foto en curso.
    Cancelled,
    /// Un error que afectaría a todas las demás; lleva el motivo.
    Stopped(String),
}

/// Cómo va o cómo acabó un lote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchReport {
    pub total: usize,
    /// Fotos ya intentadas, guardadas o saltadas.
    pub done: usize,
    pub saved: usize,
    /// Archivo creado por la última foto guardada.
    pub last: Option<PathBuf>,
    pub skipped: Vec<Skipped>,
    /// `None` mientras sigue en marcha.
    pub end: Option<BatchEnd>,
}

impl Library {
    /// Lo que hace falta para exportar `ids`, en el orden de la cuadrícula
    /// (de lo más reciente a lo más antiguo). Los ids que ya no existen se
    /// dejan fuera.
    pub fn export_items(&self, ids: &[i64]) -> Result<Vec<ExportItem>> {
        let list = serde_json::to_string(ids).expect("una lista de enteros siempre se serializa");
        let ordered: Vec<i64> = self
            .conn
            .prepare(&format!(
                "SELECT id FROM photos WHERE id IN (SELECT value FROM json_each(?1))
                 ORDER BY {DATE} DESC, id DESC"
            ))?
            .query_map([list], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut items = Vec::with_capacity(ordered.len());
        for id in ordered {
            if let Some(media) = self.photo_media(id)? {
                let name = file_name(media.rel_path.as_str()).to_owned();
                items.push(ExportItem { id, name, media });
            }
        }
        Ok(items)
    }
}

/// Exporta `items` de la biblioteca de `root` a `dest_dir`, una detrás de
/// otra. Llama a `progress` antes de empezar y tras cada foto, y comprueba
/// `cancel` entre foto y foto. Solo devuelve `Err` si no se puede empezar
/// (destino que no existe o dentro de la biblioteca); lo demás va en el
/// informe.
pub fn export_many(
    root: &Path,
    items: &[ExportItem],
    dest_dir: &Path,
    size: ExportSize,
    cancel: &AtomicBool,
    mut progress: impl FnMut(&BatchReport),
) -> Result<BatchReport> {
    if !fs::metadata(dest_dir)?.is_dir() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            format!("{} no es una carpeta", dest_dir.display()),
        )));
    }
    if inside(root, dest_dir)? {
        return Err(Error::DestinationInsideLibrary(dest_dir.to_owned()));
    }
    let mut report = BatchReport {
        total: items.len(),
        done: 0,
        saved: 0,
        last: None,
        skipped: Vec::new(),
        end: None,
    };
    progress(&report);
    for item in items {
        if cancel.load(Ordering::Relaxed) {
            report.end = Some(BatchEnd::Cancelled);
            break;
        }
        match export(root, &item.media, dest_dir, size) {
            Ok(path) => {
                report.saved += 1;
                report.last = Some(path);
            }
            Err(err) => match skip_reason(root, &err) {
                Some(reason) => report.skipped.push(Skipped {
                    name: item.name.clone(),
                    reason,
                }),
                None => {
                    report.done += 1;
                    report.end = Some(BatchEnd::Stopped(stop_reason(root, &err)));
                    break;
                }
            },
        }
        report.done += 1;
        progress(&report);
    }
    report.end.get_or_insert(BatchEnd::Finished);
    Ok(report)
}

/// Si el error es solo de esta foto, el motivo para el resumen; si afectaría
/// a las demás, `None`.
fn skip_reason(root: &Path, err: &Error) -> Option<String> {
    Some(match err {
        Error::OriginalMissing(_) if library_present(root) => {
            "El archivo no está en el disco".into()
        }
        Error::CannotReduce(_) => "Solo se puede exportar en tamaño original".into(),
        Error::VideoNotReducible(_) => "Los vídeos solo se exportan en tamaño original".into(),
        Error::CannotRender { message, .. } => {
            format!("No se puede aplicar la edición: {message}")
        }
        Error::CopyMismatch(_) => "La copia no coincidía con el original y se ha descartado".into(),
        Error::NonUtf8Path(_) => "El nombre del archivo no se puede leer".into(),
        Error::Recipe(e) => format!("La edición guardada no es válida: {e}"),
        _ => return None,
    })
}

fn stop_reason(root: &Path, err: &Error) -> String {
    if !library_present(root) {
        return "El disco de la biblioteca se ha desconectado".into();
    }
    let text = err.to_string();
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => text,
    }
}

/// La carpeta de originales sigue ahí: si no, el disco se ha ido.
fn library_present(root: &Path) -> bool {
    root.join(ORIGINALS_DIR).is_dir()
}

fn file_name(rel_path: &str) -> &str {
    rel_path.rsplit('/').next().unwrap_or(rel_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Setup {
        dir: tempfile::TempDir,
        lib: Library,
        dest: PathBuf,
    }

    /// Biblioteca con las fotos `names` (JPEG pequeños, salvo los `.heic`,
    /// que no se pueden decodificar) y una carpeta de destino fuera.
    fn setup(names: &[&str]) -> Setup {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Biblioteca");
        let mut lib = Library::create(&root).unwrap();
        for name in names {
            let path = root.join(ORIGINALS_DIR).join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            if name.ends_with(".heic") {
                fs::write(&path, b"no es un heic de verdad").unwrap();
            } else {
                image::RgbImage::from_pixel(8, 8, image::Rgb([200, 40, 40]))
                    .save_with_format(&path, image::ImageFormat::Jpeg)
                    .unwrap();
            }
        }
        lib.index().unwrap();
        let dest = dir.path().join("Para el pendrive ñ");
        fs::create_dir(&dest).unwrap();
        Setup { dir, lib, dest }
    }

    fn all_items(lib: &Library) -> Vec<ExportItem> {
        let ids: Vec<i64> = lib
            .connection()
            .prepare("SELECT id FROM photos")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        lib.export_items(&ids).unwrap()
    }

    fn dest_names(dest: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dest)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    /// Contenido y fecha de cada original, y las filas de la base de datos.
    fn snapshot(lib: &Library) -> (Vec<(Vec<u8>, std::time::SystemTime)>, Vec<String>) {
        let rows: Vec<String> = lib
            .connection()
            .prepare(
                "SELECT rel_path || size_bytes || modified_at || content_hash FROM photos ORDER BY id",
            )
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let mut files: Vec<PathBuf> = walk(&lib.root().join(ORIGINALS_DIR));
        files.sort();
        let files = files
            .iter()
            .map(|p| {
                (
                    fs::read(p).unwrap(),
                    fs::metadata(p).unwrap().modified().unwrap(),
                )
            })
            .collect();
        (files, rows)
    }

    fn walk(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else {
                out.push(path);
            }
        }
        out
    }

    #[test]
    fn exporta_todas_sin_pisar_nombres_repetidos() {
        let s = setup(&["2023/IMG_0001.jpg", "2024/IMG_0001.jpg", "playa.jpg"]);
        fs::write(s.dest.join("playa.jpg"), b"ya estaba").unwrap();
        let antes = snapshot(&s.lib);
        let items = all_items(&s.lib);
        let mut avances = Vec::new();
        let report = export_many(
            s.lib.root(),
            &items,
            &s.dest,
            ExportSize::Original,
            &AtomicBool::new(false),
            |r| avances.push(r.done),
        )
        .unwrap();
        assert_eq!(report.end, Some(BatchEnd::Finished));
        assert_eq!((report.total, report.done, report.saved), (3, 3, 3));
        assert!(report.skipped.is_empty());
        assert_eq!(avances, [0, 1, 2, 3]);
        assert_eq!(
            dest_names(&s.dest),
            [
                "IMG_0001 (1).jpg",
                "IMG_0001.jpg",
                "playa (1).jpg",
                "playa.jpg"
            ]
        );
        assert_eq!(fs::read(s.dest.join("playa.jpg")).unwrap(), b"ya estaba");
        assert_eq!(snapshot(&s.lib), antes);
    }

    #[test]
    fn salta_las_que_faltan_o_no_se_reducen_y_sigue() {
        let s = setup(&["a.jpg", "movil.heic", "b.jpg", "c.jpg"]);
        fs::remove_file(s.lib.root().join("Originales/b.jpg")).unwrap();
        let items = all_items(&s.lib);
        let report = export_many(
            s.lib.root(),
            &items,
            &s.dest,
            ExportSize::Reduced,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
        assert_eq!(report.end, Some(BatchEnd::Finished));
        assert_eq!((report.done, report.saved), (4, 2));
        let mut skipped = report.skipped.clone();
        skipped.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(
            skipped,
            [
                Skipped {
                    name: "b.jpg".into(),
                    reason: "El archivo no está en el disco".into()
                },
                Skipped {
                    name: "movil.heic".into(),
                    reason: "Solo se puede exportar en tamaño original".into()
                },
            ]
        );
        assert_eq!(dest_names(&s.dest), ["a.jpg", "c.jpg"]);
    }

    #[test]
    fn un_error_del_destino_detiene_el_lote() {
        let s = setup(&["a.jpg", "b.jpg", "c.jpg"]);
        let items = all_items(&s.lib);
        let dest = s.dest.clone();
        // El destino desaparece tras la primera foto, como un pendrive que
        // se quita.
        let report = export_many(
            s.lib.root(),
            &items,
            &s.dest,
            ExportSize::Original,
            &AtomicBool::new(false),
            |r| {
                if r.done == 1 {
                    fs::rename(&dest, s.dir.path().join("quitado")).unwrap();
                }
            },
        )
        .unwrap();
        assert!(
            matches!(report.end, Some(BatchEnd::Stopped(_))),
            "{report:?}"
        );
        assert_eq!((report.done, report.saved), (2, 1));
        assert!(report.skipped.is_empty());
        assert_eq!(dest_names(&s.dir.path().join("quitado")).len(), 1);
    }

    #[test]
    fn si_se_va_la_biblioteca_se_detiene_en_vez_de_saltar_todas() {
        let s = setup(&["a.jpg", "b.jpg", "c.jpg"]);
        let items = all_items(&s.lib);
        let root = s.lib.root().to_owned();
        let report = export_many(
            &root,
            &items,
            &s.dest,
            ExportSize::Original,
            &AtomicBool::new(false),
            |r| {
                if r.done == 1 {
                    fs::rename(root.join(ORIGINALS_DIR), root.join("fuera")).unwrap();
                }
            },
        )
        .unwrap();
        assert_eq!(
            report.end,
            Some(BatchEnd::Stopped(
                "El disco de la biblioteca se ha desconectado".into()
            ))
        );
        assert_eq!((report.done, report.saved), (2, 1));
        assert!(report.skipped.is_empty());
    }

    #[test]
    fn cancelar_para_tras_la_foto_en_curso() {
        let s = setup(&["a.jpg", "b.jpg", "c.jpg", "d.jpg"]);
        let items = all_items(&s.lib);
        let cancel = AtomicBool::new(false);
        let report = export_many(
            s.lib.root(),
            &items,
            &s.dest,
            ExportSize::Reduced,
            &cancel,
            |r| {
                if r.done == 2 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
        )
        .unwrap();
        assert_eq!(report.end, Some(BatchEnd::Cancelled));
        assert_eq!((report.done, report.saved), (2, 2));
        assert_eq!(dest_names(&s.dest).len(), 2);
        // Lo exportado está entero: se puede decodificar.
        for name in dest_names(&s.dest) {
            image::open(s.dest.join(name)).unwrap();
        }
    }

    #[test]
    fn rechaza_un_destino_dentro_de_la_biblioteca_antes_de_empezar() {
        let s = setup(&["a.jpg"]);
        let items = all_items(&s.lib);
        let mut llamadas = 0;
        let err = export_many(
            s.lib.root(),
            &items,
            &s.lib.root().join(ORIGINALS_DIR),
            ExportSize::Original,
            &AtomicBool::new(false),
            |_| llamadas += 1,
        )
        .unwrap_err();
        assert!(matches!(err, Error::DestinationInsideLibrary(_)), "{err}");
        assert_eq!(llamadas, 0);
    }

    #[test]
    fn la_foto_editada_sale_igual_que_desde_el_visor() {
        use mnemos_imaging::recipe::{Crop, Geometry};
        let mut s = setup(&["a.jpg"]);
        let id = s
            .lib
            .connection()
            .query_row("SELECT id FROM photos", [], |r| r.get(0))
            .unwrap();
        s.lib
            .save_edit(
                id,
                Geometry {
                    rotate: 90,
                    flip_horizontal: false,
                    crop: Some(Crop {
                        x: 0.0,
                        y: 0.0,
                        width: 0.5,
                        height: 1.0,
                    }),
                },
            )
            .unwrap();
        let visor = tempfile::tempdir().unwrap();
        let una = s
            .lib
            .export_photo(id, visor.path(), ExportSize::Original)
            .unwrap();
        let report = export_many(
            s.lib.root(),
            &s.lib.export_items(&[id]).unwrap(),
            &s.dest,
            ExportSize::Original,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
        assert_eq!(report.saved, 1);
        assert_eq!(
            fs::read(report.last.unwrap()).unwrap(),
            fs::read(una).unwrap()
        );
    }

    #[test]
    fn los_ids_salen_en_el_orden_de_la_cuadricula_y_sin_los_que_no_existen() {
        let s = setup(&["a.jpg", "b.jpg", "c.jpg"]);
        let conn = s.lib.connection();
        for (name, taken) in [("a.jpg", 100), ("b.jpg", 300), ("c.jpg", 200)] {
            conn.execute(
                "UPDATE photos SET taken_at = ?1 WHERE rel_path = ?2",
                rusqlite::params![taken, format!("Originales/{name}")],
            )
            .unwrap();
        }
        let ids: Vec<i64> = conn
            .prepare("SELECT id FROM photos ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let mut pedidos = ids.clone();
        pedidos.push(9999);
        let names: Vec<String> = s
            .lib
            .export_items(&pedidos)
            .unwrap()
            .into_iter()
            .map(|i| i.name)
            .collect();
        assert_eq!(names, ["b.jpg", "c.jpg", "a.jpg"]);
    }
}
