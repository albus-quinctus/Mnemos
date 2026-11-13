//! Exportar varias fotos a la vez desde la barra de selección (spec 017).
//!
//! El lote va en un hilo aparte: la biblioteca solo se bloquea para traducir
//! los ids, y los avances llegan a la interfaz con el evento `exportacion`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mnemos_core::ExportSize;
use mnemos_core::export_many::{BatchEnd, BatchReport, export_many};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{OpenLibrary, lock, with_library};

/// La exportación en marcha, si la hay. Guarda la señal para pararla.
#[derive(Default)]
pub struct Exportacion(Mutex<Option<Arc<AtomicBool>>>);

/// Cada cuánto, como mucho, se avisa del progreso.
const INTERVALO: Duration = Duration::from_millis(100);

/// Una foto que no se pudo exportar.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Saltada {
    nombre: String,
    motivo: String,
}

/// Lo que la interfaz recibe sobre una exportación. `estado` puede ser
/// `en_curso`, `terminada`, `cancelada` o `detenida`.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct EventoExportacion {
    estado: &'static str,
    total: usize,
    hechas: usize,
    guardadas: usize,
    carpeta: String,
    /// Ruta del archivo creado cuando el lote es de una sola foto.
    ruta: Option<String>,
    saltadas: Vec<Saltada>,
    mensaje: Option<String>,
}

impl EventoExportacion {
    fn de(informe: &BatchReport, carpeta: &Path) -> Self {
        let (estado, mensaje) = match &informe.end {
            None => ("en_curso", None),
            Some(BatchEnd::Finished) => ("terminada", None),
            Some(BatchEnd::Cancelled) => ("cancelada", None),
            Some(BatchEnd::Stopped(motivo)) => ("detenida", Some(motivo.clone())),
        };
        let ruta = match (&informe.end, informe.total, &informe.last) {
            (Some(BatchEnd::Finished), 1, Some(ruta)) => Some(ruta.display().to_string()),
            _ => None,
        };
        Self {
            estado,
            total: informe.total,
            hechas: informe.done,
            guardadas: informe.saved,
            carpeta: carpeta.display().to_string(),
            ruta,
            saltadas: informe
                .skipped
                .iter()
                .map(|s| Saltada {
                    nombre: s.name.clone(),
                    motivo: s.reason.clone(),
                })
                .collect(),
            mensaje,
        }
    }

    fn fallida(total: usize, carpeta: &Path, mensaje: String) -> Self {
        Self {
            estado: "detenida",
            total,
            hechas: 0,
            guardadas: 0,
            carpeta: carpeta.display().to_string(),
            ruta: None,
            saltadas: Vec::new(),
            mensaje: Some(mayuscula(&mensaje)),
        }
    }
}

fn mayuscula(texto: &str) -> String {
    let mut letras = texto.chars();
    match letras.next() {
        Some(primera) => primera.to_uppercase().chain(letras).collect(),
        None => String::new(),
    }
}

/// Empieza a exportar `ids` a `carpeta` en segundo plano. Solo puede haber
/// una exportación de varias a la vez; la del visor va aparte.
#[tauri::command]
pub fn exportar_varias(
    ids: Vec<i64>,
    carpeta: String,
    tamano: ExportSize,
    app: AppHandle,
    estado: State<'_, OpenLibrary>,
    exportacion: State<'_, Exportacion>,
) -> Result<(), String> {
    let (root, items) = with_library(&estado, |lib| {
        Ok((lib.root().to_path_buf(), lib.export_items(&ids)?))
    })?;
    let cancelar = Arc::new(AtomicBool::new(false));
    {
        let mut activa = lock(&exportacion.0);
        if activa.is_some() {
            return Err("ya hay una exportación en marcha".to_owned());
        }
        *activa = Some(Arc::clone(&cancelar));
    }
    let carpeta = PathBuf::from(carpeta);
    let emisor = app.clone();
    std::thread::spawn(move || {
        let mut ultimo: Option<Instant> = None;
        let resultado = export_many(&root, &items, &carpeta, tamano, &cancelar, |informe| {
            if ultimo.is_some_and(|t| t.elapsed() < INTERVALO) {
                return;
            }
            ultimo = Some(Instant::now());
            let _ = emisor.emit("exportacion", EventoExportacion::de(informe, &carpeta));
        });
        let evento = match resultado {
            Ok(informe) => EventoExportacion::de(&informe, &carpeta),
            Err(e) => EventoExportacion::fallida(items.len(), &carpeta, e.to_string()),
        };
        // Se libera antes de avisar, para que se pueda empezar otra en cuanto
        // la interfaz sepa que esta ha acabado.
        lock(&emisor.state::<Exportacion>().0).take();
        let _ = emisor.emit("exportacion", evento);
    });
    Ok(())
}

/// Pide parar la exportación en marcha tras la foto en curso.
#[tauri::command]
pub fn cancelar_exportacion(exportacion: State<'_, Exportacion>) {
    if let Some(cancelar) = lock(&exportacion.0).as_ref() {
        cancelar.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mnemos_core::export_many::Skipped;

    fn informe(end: Option<BatchEnd>, total: usize, saved: usize) -> BatchReport {
        BatchReport {
            total,
            done: saved,
            saved,
            last: Some(PathBuf::from("/destino/foto.jpg")),
            skipped: Vec::new(),
            end,
        }
    }

    #[test]
    fn el_estado_sale_del_final_del_lote() {
        let carpeta = Path::new("/destino");
        let estados: Vec<_> = [
            None,
            Some(BatchEnd::Finished),
            Some(BatchEnd::Cancelled),
            Some(BatchEnd::Stopped("Sin espacio".into())),
        ]
        .into_iter()
        .map(|end| EventoExportacion::de(&informe(end, 3, 2), carpeta))
        .map(|e| (e.estado, e.mensaje))
        .collect();
        assert_eq!(
            estados,
            [
                ("en_curso", None),
                ("terminada", None),
                ("cancelada", None),
                ("detenida", Some("Sin espacio".to_owned())),
            ]
        );
    }

    #[test]
    fn la_ruta_solo_va_con_una_sola_foto_terminada() {
        let carpeta = Path::new("/destino");
        let una = EventoExportacion::de(&informe(Some(BatchEnd::Finished), 1, 1), carpeta);
        assert_eq!(una.ruta.as_deref(), Some("/destino/foto.jpg"));
        let varias = EventoExportacion::de(&informe(Some(BatchEnd::Finished), 2, 2), carpeta);
        assert_eq!(varias.ruta, None);
    }

    #[test]
    fn las_saltadas_llegan_con_nombre_y_motivo() {
        let mut i = informe(Some(BatchEnd::Finished), 2, 1);
        i.skipped.push(Skipped {
            name: "movil.heic".into(),
            reason: "Solo se puede exportar en tamaño original".into(),
        });
        let json = serde_json::to_value(EventoExportacion::de(&i, Path::new("/d"))).unwrap();
        assert_eq!(
            json["saltadas"],
            serde_json::json!([{
                "nombre": "movil.heic",
                "motivo": "Solo se puede exportar en tamaño original"
            }])
        );
        assert_eq!(json["hechas"], 1);
    }

    #[test]
    fn si_no_puede_empezar_avisa_con_mayuscula() {
        let e = EventoExportacion::fallida(
            3,
            Path::new("/d"),
            "no se puede exportar en /d: está dentro de la biblioteca".into(),
        );
        assert_eq!(e.estado, "detenida");
        assert!(e.mensaje.unwrap().starts_with("No se puede"));
    }
}
