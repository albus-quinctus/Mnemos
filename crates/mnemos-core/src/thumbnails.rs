//! Miniaturas en disco y su cola de generación en segundo plano (spec 002).
//!
//! Cada miniatura se guarda en `.mnemos/miniaturas/v1/<ab>/<huella>.jpg`: al ir
//! por la huella, dos copias iguales comparten miniatura y mover o renombrar
//! una foto no obliga a regenerarla. Se escribe primero en `.mnemos/tmp/` y
//! luego se mueve a su sitio, así que un corte nunca deja una a medias.

use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::db;
use crate::error::Result;
use crate::library::{DATA_DIR, DB_FILE, Library, TMP_DIR};
use crate::paths::RelPath;

/// Carpeta de las miniaturas, relativa a la raíz. `v1` cambia si cambia su
/// tamaño o su calidad, para regenerarlas todas.
pub const THUMBNAILS_DIR: &str = ".mnemos/miniaturas/v1";

/// Hilos por defecto: en un HDD leer muchos archivos a la vez es más lento.
pub const DEFAULT_WORKERS: usize = 2;

/// Foto cuya miniatura falta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailJob {
    pub photo_id: i64,
    pub rel_path: RelPath,
    pub content_hash: String,
    pub orientation: u16,
}

/// Lo que pasa con cada trabajo de la cola.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum ThumbnailEvent {
    /// La miniatura está lista. Lleva todas las fotos que la comparten.
    Ready { photo_ids: Vec<i64> },
    /// No se pudo generar y no se volverá a intentar hasta que cambie el archivo.
    Failed {
        photo_ids: Vec<i64>,
        message: String,
    },
    /// La cola se ha parado porque la biblioteca ya no está accesible, por
    /// ejemplo porque se desconectó el disco. Se reanuda abriéndola de nuevo.
    Stopped { message: String },
}

/// Resultado de generar una miniatura.
#[derive(Debug)]
pub enum Outcome {
    Written,
    AlreadyThere,
    /// El formato no tiene miniatura o la imagen está dañada.
    Failed(String),
}

/// Ruta relativa de la miniatura de una foto con esta huella.
pub fn thumbnail_rel_path(content_hash: &str) -> String {
    let prefix = content_hash.get(..2).unwrap_or("00");
    format!("{THUMBNAILS_DIR}/{prefix}/{content_hash}.jpg")
}

impl Library {
    /// Ruta absoluta de la miniatura de una foto con esta huella.
    pub fn thumbnail_path(&self, content_hash: &str) -> PathBuf {
        self.root.join(thumbnail_rel_path(content_hash))
    }

    /// Fotos sin miniatura en disco, sin contar las que ya fallaron con su
    /// contenido actual, en el orden de la cuadrícula. Si dos fotos comparten
    /// huella, solo sale una.
    pub fn pending_thumbnails(&self) -> Result<Vec<ThumbnailJob>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.rel_path, p.content_hash, coalesce(p.orientation, 1)
             FROM photos p
             LEFT JOIN thumbnail_failures f ON f.content_hash = p.content_hash
             WHERE p.content_hash IS NOT NULL AND f.content_hash IS NULL
             ORDER BY coalesce(p.taken_at, p.modified_at) DESC, p.id DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, u16>(3)?,
            ))
        })?;
        let mut seen = HashSet::new();
        let mut jobs = Vec::new();
        for row in rows {
            let (photo_id, rel_path, content_hash, orientation) = row?;
            if !seen.insert(content_hash.clone()) || self.thumbnail_path(&content_hash).is_file() {
                continue;
            }
            jobs.push(ThumbnailJob {
                photo_id,
                rel_path: RelPath::new(&rel_path)?,
                content_hash,
                orientation,
            });
        }
        Ok(jobs)
    }

    /// Genera ahora, en este hilo, todas las miniaturas que faltan.
    pub fn generate_thumbnails(&self) -> Result<ThumbnailReport> {
        let mut report = ThumbnailReport::default();
        for job in self.pending_thumbnails()? {
            let path = job.rel_path.as_str().to_owned();
            match make_thumbnail(&self.root, &job) {
                Ok(Outcome::Written) => report.written += 1,
                Ok(Outcome::AlreadyThere) => {}
                Ok(Outcome::Failed(message)) => {
                    record_failure(&self.conn, &job.content_hash, &message)?;
                    report.failed.push((path, message));
                }
                // La biblioteca ya no está (disco desconectado): se para todo.
                Err(err) if !self.root.join(DATA_DIR).is_dir() => return Err(err.into()),
                // El original falta o no se puede leer: no se anota, se
                // reintenta la próxima vez.
                Err(err) => report.failed.push((path, err.to_string())),
            }
        }
        Ok(report)
    }
}

/// Resumen de [`Library::generate_thumbnails`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct ThumbnailReport {
    pub written: u64,
    /// Ruta relativa y motivo de cada foto sin miniatura.
    pub failed: Vec<(String, String)>,
}

/// Genera y guarda la miniatura de un trabajo. Un error de lectura o escritura
/// se devuelve como `Err` (puede ser un disco desconectado); un formato sin
/// miniatura o una imagen dañada, como [`Outcome::Failed`].
pub fn make_thumbnail(root: &Path, job: &ThumbnailJob) -> io::Result<Outcome> {
    let dest = root.join(thumbnail_rel_path(&job.content_hash));
    if dest.is_file() {
        return Ok(Outcome::AlreadyThere);
    }
    let thumb = match mnemos_imaging::thumbnail(&job.rel_path.to_path(root), job.orientation) {
        Ok(thumb) => thumb,
        Err(mnemos_imaging::ThumbnailError::Io(err)) => return Err(err),
        Err(err) => return Ok(Outcome::Failed(err.to_string())),
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let tmp_dir = root.join(DATA_DIR).join(TMP_DIR);
    fs::create_dir_all(&tmp_dir)?;
    let part = tmp_dir.join(format!(
        "{}-{}-{}.jpg.part",
        job.content_hash,
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let written = fs::write(&part, &thumb.jpeg)
        .and_then(|()| fs::File::open(&part)?.sync_all())
        .and_then(|()| fs::create_dir_all(dest.parent().unwrap_or(root)))
        .and_then(|()| fs::rename(&part, &dest));
    if let Err(err) = written {
        let _ = fs::remove_file(&part);
        return Err(err);
    }
    Ok(Outcome::Written)
}

fn record_failure(conn: &Connection, content_hash: &str, message: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO thumbnail_failures (content_hash, message, failed_at)
         VALUES (?1, ?2, unixepoch())",
        params![content_hash, message],
    )?;
    Ok(())
}

/// Ids de las fotos que comparten una huella.
fn photos_with_hash(conn: &Connection, content_hash: &str) -> rusqlite::Result<Vec<i64>> {
    let mut stmt = conn.prepare_cached("SELECT id FROM photos WHERE content_hash = ?1")?;
    stmt.query_map([content_hash], |r| r.get(0))?.collect()
}

/// Trabajos pendientes, con los prioritarios delante y sin huellas repetidas.
#[derive(Default)]
struct Jobs {
    queue: VecDeque<ThumbnailJob>,
    hashes: HashSet<String>,
    stopped: bool,
}

impl Jobs {
    fn push(&mut self, job: ThumbnailJob) {
        if self.hashes.insert(job.content_hash.clone()) {
            self.queue.push_back(job);
        }
    }

    /// Pone delante, en este orden, los trabajos de estas fotos.
    fn prioritize(&mut self, photo_ids: &[i64]) {
        for id in photo_ids.iter().rev() {
            if let Some(pos) = self.queue.iter().position(|j| j.photo_id == *id)
                && let Some(job) = self.queue.remove(pos)
            {
                self.queue.push_front(job);
            }
        }
    }

    fn pop(&mut self) -> Option<ThumbnailJob> {
        let job = self.queue.pop_front()?;
        self.hashes.remove(&job.content_hash);
        Some(job)
    }
}

struct Shared {
    root: PathBuf,
    jobs: Mutex<Jobs>,
    wake: Condvar,
    /// Conexión propia para anotar fallos y saber qué fotos comparten huella.
    conn: Mutex<Connection>,
    on_event: Box<dyn Fn(ThumbnailEvent) + Send + Sync>,
}

/// Cola de miniaturas con unos pocos hilos. Se para al soltarla.
pub struct ThumbnailQueue {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
}

impl ThumbnailQueue {
    /// Arranca `workers` hilos (al menos uno) sobre la biblioteca de `root`.
    /// `on_event` se llama desde los hilos de la cola.
    pub fn start(
        root: &Path,
        workers: usize,
        on_event: impl Fn(ThumbnailEvent) + Send + Sync + 'static,
    ) -> Result<Self> {
        let conn = db::open(&root.join(DATA_DIR).join(DB_FILE))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let shared = Arc::new(Shared {
            root: root.to_owned(),
            jobs: Mutex::new(Jobs::default()),
            wake: Condvar::new(),
            conn: Mutex::new(conn),
            on_event: Box::new(on_event),
        });
        let workers = (0..workers.max(1))
            .map(|_| {
                let shared = Arc::clone(&shared);
                std::thread::spawn(move || work(&shared))
            })
            .collect();
        Ok(Self { shared, workers })
    }

    /// Añade trabajos al final de la cola.
    pub fn push(&self, jobs: impl IntoIterator<Item = ThumbnailJob>) {
        let mut queue = lock(&self.shared.jobs);
        for job in jobs {
            queue.push(job);
        }
        self.shared.wake.notify_all();
    }

    /// Adelanta las fotos que se ven en pantalla, en el orden dado.
    pub fn prioritize(&self, photo_ids: &[i64]) {
        lock(&self.shared.jobs).prioritize(photo_ids);
    }

    /// Trabajos que aún esperan.
    pub fn pending(&self) -> usize {
        lock(&self.shared.jobs).queue.len()
    }

    /// Espera a que se vacíe la cola y se paran los hilos. Los trabajos en
    /// curso terminan.
    pub fn finish(mut self) {
        lock(&self.shared.jobs).stopped = true;
        self.shared.wake.notify_all();
        self.join();
    }

    fn join(&mut self) {
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl Drop for ThumbnailQueue {
    /// Al soltarla, la cola se para sin esperar a los trabajos pendientes.
    fn drop(&mut self) {
        {
            let mut jobs = lock(&self.shared.jobs);
            jobs.queue.clear();
            jobs.stopped = true;
        }
        self.shared.wake.notify_all();
        self.join();
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // Un pánico en otro hilo no debe bloquear la cola para siempre.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn work(shared: &Shared) {
    loop {
        let job = {
            let mut jobs = lock(&shared.jobs);
            loop {
                if let Some(job) = jobs.pop() {
                    break job;
                }
                if jobs.stopped {
                    return;
                }
                jobs = shared.wake.wait(jobs).unwrap_or_else(|e| e.into_inner());
            }
        };
        let event = match make_thumbnail(&shared.root, &job) {
            Ok(Outcome::Written | Outcome::AlreadyThere) => ThumbnailEvent::Ready {
                photo_ids: sharing(shared, &job),
            },
            Ok(Outcome::Failed(message)) => {
                let _ = record_failure(&lock(&shared.conn), &job.content_hash, &message);
                ThumbnailEvent::Failed {
                    photo_ids: sharing(shared, &job),
                    message,
                }
            }
            Err(err) if !shared.root.join(DATA_DIR).is_dir() => {
                // La biblioteca ya no está: se para la cola entera.
                let mut jobs = lock(&shared.jobs);
                jobs.queue.clear();
                jobs.stopped = true;
                shared.wake.notify_all();
                drop(jobs);
                (shared.on_event)(ThumbnailEvent::Stopped {
                    message: err.to_string(),
                });
                return;
            }
            // El original falta o no se puede leer: se reintenta en otra sesión.
            Err(err) => ThumbnailEvent::Failed {
                photo_ids: vec![job.photo_id],
                message: err.to_string(),
            },
        };
        (shared.on_event)(event);
    }
}

fn sharing(shared: &Shared, job: &ThumbnailJob) -> Vec<i64> {
    photos_with_hash(&lock(&shared.conn), &job.content_hash)
        .ok()
        .filter(|ids| !ids.is_empty())
        .unwrap_or_else(|| vec![job.photo_id])
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(dir.path()).unwrap();
        (dir, lib)
    }

    /// Escribe una imagen PNG que se puede decodificar.
    fn write_image(path: &Path, width: u32, height: u32, shade: u8) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbImage::from_pixel(width, height, image::Rgb([shade, 100, 50]))
            .save(path)
            .unwrap();
    }

    fn job(id: i64, hash: &str) -> ThumbnailJob {
        ThumbnailJob {
            photo_id: id,
            rel_path: RelPath::new("Originales/x.png").unwrap(),
            content_hash: hash.to_owned(),
            orientation: 1,
        }
    }

    #[test]
    fn la_ruta_va_por_la_huella() {
        assert_eq!(
            thumbnail_rel_path("abcdef"),
            ".mnemos/miniaturas/v1/ab/abcdef.jpg"
        );
    }

    #[test]
    fn genera_las_que_faltan_una_sola_vez() {
        let (dir, mut lib) = library();
        let originals = dir.path().join("Originales");
        write_image(&originals.join("a.png"), 800, 600, 1);
        write_image(&originals.join("b.png"), 600, 800, 2);
        // Copia idéntica de a.png: comparte miniatura.
        fs::copy(originals.join("a.png"), originals.join("copia.png")).unwrap();
        lib.index().unwrap();

        assert_eq!(lib.pending_thumbnails().unwrap().len(), 2);
        let report = lib.generate_thumbnails().unwrap();
        assert_eq!(report.written, 2);
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        assert!(lib.pending_thumbnails().unwrap().is_empty());

        let thumbs: Vec<_> = walkdir::WalkDir::new(dir.path().join(THUMBNAILS_DIR))
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();
        assert_eq!(thumbs.len(), 2);
        let tmp = dir.path().join(".mnemos/tmp");
        assert_eq!(fs::read_dir(tmp).map(|d| d.count()).unwrap_or(0), 0);

        assert_eq!(lib.generate_thumbnails().unwrap().written, 0);
    }

    #[test]
    fn un_fallo_se_anota_y_no_se_repite_hasta_que_cambia_el_archivo() {
        let (dir, mut lib) = library();
        let rota = dir.path().join("Originales/rota.png");
        fs::create_dir_all(rota.parent().unwrap()).unwrap();
        fs::write(&rota, b"\x89PNG\r\n\x1a\nroto").unwrap();
        lib.index().unwrap();

        let report = lib.generate_thumbnails().unwrap();
        assert_eq!(report.failed.len(), 1);
        assert!(lib.pending_thumbnails().unwrap().is_empty());

        write_image(&rota, 10, 10, 3);
        let file = fs::File::options().write(true).open(&rota).unwrap();
        file.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(60))
            .unwrap();
        lib.index().unwrap();
        assert_eq!(lib.pending_thumbnails().unwrap().len(), 1);
    }

    #[test]
    fn no_modifica_los_originales() {
        let (dir, mut lib) = library();
        let foto = dir.path().join("Originales/foto.png");
        write_image(&foto, 640, 480, 4);
        let before = fs::read(&foto).unwrap();
        let mtime = fs::metadata(&foto).unwrap().modified().unwrap();
        lib.index().unwrap();
        lib.generate_thumbnails().unwrap();
        assert_eq!(fs::read(&foto).unwrap(), before);
        assert_eq!(fs::metadata(&foto).unwrap().modified().unwrap(), mtime);
    }

    #[test]
    fn la_prioridad_adelanta_las_fotos_en_pantalla() {
        let mut jobs = Jobs::default();
        for (id, hash) in [(1, "a"), (2, "b"), (3, "c"), (4, "d")] {
            jobs.push(job(id, hash));
        }
        jobs.push(job(5, "a")); // misma huella que la 1: no se repite
        jobs.prioritize(&[3, 99, 2]);
        let order: Vec<i64> = std::iter::from_fn(|| jobs.pop())
            .map(|j| j.photo_id)
            .collect();
        assert_eq!(order, [3, 2, 1, 4]);
    }

    #[test]
    fn la_cola_genera_en_segundo_plano_y_avisa() {
        let (dir, mut lib) = library();
        let originals = dir.path().join("Originales");
        for i in 0..5u8 {
            write_image(&originals.join(format!("{i}.png")), 300, 200, i);
        }
        fs::write(originals.join("rota.png"), b"no es png").unwrap();
        lib.index().unwrap();

        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let queue = ThumbnailQueue::start(dir.path(), DEFAULT_WORKERS, move |event| {
            lock(&tx).send(event).unwrap();
        })
        .unwrap();
        queue.push(lib.pending_thumbnails().unwrap());
        queue.finish();

        let events: Vec<_> = rx.try_iter().collect();
        let ready = events
            .iter()
            .filter(|e| matches!(e, ThumbnailEvent::Ready { .. }))
            .count();
        let failed = events
            .iter()
            .filter(|e| matches!(e, ThumbnailEvent::Failed { .. }))
            .count();
        assert_eq!((ready, failed), (5, 1), "{events:?}");
        // El fallo quedó anotado por la conexión de la cola.
        assert!(lib.pending_thumbnails().unwrap().is_empty());
    }

    #[test]
    fn la_cola_se_para_si_desaparece_la_biblioteca() {
        let (dir, mut lib) = library();
        write_image(&dir.path().join("Originales/a.png"), 50, 50, 1);
        lib.index().unwrap();
        let jobs = lib.pending_thumbnails().unwrap();
        drop(lib);

        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let queue = ThumbnailQueue::start(dir.path(), 1, move |event| {
            lock(&tx).send(event).unwrap();
        })
        .unwrap();
        // Simula un disco desconectado: la biblioteca ya no está.
        fs::remove_dir_all(dir.path().join(".mnemos")).unwrap();
        fs::remove_dir_all(dir.path().join("Originales")).unwrap();
        queue.push(jobs);
        queue.finish();
        let events: Vec<_> = rx.try_iter().collect();
        assert!(
            matches!(events.as_slice(), [ThumbnailEvent::Stopped { .. }]),
            "{events:?}"
        );
    }
}
