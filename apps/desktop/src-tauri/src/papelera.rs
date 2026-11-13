//! Comandos de la Papelera (spec 015).
//!
//! Enviar y restaurar solo cambian la base de datos. Eliminar para siempre,
//! vaciar y el vaciado automático borran originales (la excepción a la regla
//! 1): van en segundo plano, con una conexión propia para no bloquear la
//! biblioteca abierta, y avisan con eventos `papelera`. Solo puede haber uno
//! a la vez.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use mnemos_core::{AutoEmpty, DeleteReport, Library};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{OpenLibrary, lock, with_library, with_library_mut};

/// Si hay un borrado en marcha.
#[derive(Default)]
pub struct Borrado(Mutex<bool>);

/// Qué se está borrando.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Motivo {
    /// «Eliminar para siempre» sobre una selección.
    Seleccion,
    /// «Vaciar papelera».
    Vaciar,
    /// Lo que lleva 30 días, al abrir la biblioteca o cada hora.
    Automatico,
}

/// Una foto que no se pudo eliminar y sigue en la Papelera.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fallida {
    pub ruta: String,
    pub motivo: String,
}

/// Lo que la interfaz recibe sobre un borrado. `estado` puede ser
/// `en_curso`, `terminado`, `reloj_adelantado` o `fallido`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventoPapelera {
    pub estado: &'static str,
    pub motivo: Motivo,
    pub hechas: usize,
    pub total: usize,
    /// Ids eliminados, para quitarlos de la cuadrícula.
    pub eliminadas: Vec<i64>,
    pub fallidas: Vec<Fallida>,
    /// Con `reloj_adelantado`: cuántas fotos se eliminarían si el usuario
    /// confirma que la fecha del equipo está bien.
    pub caducadas: u64,
    pub mensaje: Option<String>,
}

impl EventoPapelera {
    fn nuevo(estado: &'static str, motivo: Motivo) -> Self {
        Self {
            estado,
            motivo,
            hechas: 0,
            total: 0,
            eliminadas: Vec::new(),
            fallidas: Vec::new(),
            caducadas: 0,
            mensaje: None,
        }
    }

    fn en_curso(motivo: Motivo, hechas: usize, total: usize) -> Self {
        Self {
            hechas,
            total,
            ..Self::nuevo("en_curso", motivo)
        }
    }

    fn terminado(motivo: Motivo, informe: DeleteReport) -> Self {
        let total = informe.deleted.len() + informe.failed.len();
        Self {
            hechas: total,
            total,
            eliminadas: informe.deleted,
            fallidas: informe
                .failed
                .into_iter()
                .map(|f| Fallida {
                    ruta: f.rel_path,
                    motivo: f.reason,
                })
                .collect(),
            ..Self::nuevo("terminado", motivo)
        }
    }

    fn fallido(motivo: Motivo, mensaje: String) -> Self {
        Self {
            mensaje: Some(mensaje),
            ..Self::nuevo("fallido", motivo)
        }
    }
}

/// Envía las fotos a la Papelera. No toca los archivos. Devuelve los ids que
/// han cambiado, para «Deshacer».
#[tauri::command]
pub fn enviar_a_papelera(
    ids: Vec<i64>,
    estado: State<'_, OpenLibrary>,
) -> Result<Vec<i64>, String> {
    with_library_mut(&estado, |lib| lib.send_to_trash(&ids))
}

/// Saca las fotos de la Papelera. Devuelve los ids que han cambiado.
#[tauri::command]
pub fn restaurar(ids: Vec<i64>, estado: State<'_, OpenLibrary>) -> Result<Vec<i64>, String> {
    with_library_mut(&estado, |lib| lib.restore_from_trash(&ids))
}

/// Elimina para siempre las fotos de la selección que estén en la Papelera.
/// La interfaz ya ha pedido confirmación.
#[tauri::command]
pub fn eliminar_para_siempre(
    ids: Vec<i64>,
    app: AppHandle,
    estado: State<'_, OpenLibrary>,
) -> Result<(), String> {
    let root = raiz(&estado)?;
    en_segundo_plano(
        &app,
        Motivo::Seleccion,
        move |lib, avance| lib.delete_permanently(&ids, avance),
        root,
    )
}

/// Elimina para siempre todo lo que hay en la Papelera. La interfaz ya ha
/// pedido confirmación.
#[tauri::command]
pub fn vaciar_papelera(app: AppHandle, estado: State<'_, OpenLibrary>) -> Result<(), String> {
    let root = raiz(&estado)?;
    en_segundo_plano(
        &app,
        Motivo::Vaciar,
        |lib, avance| {
            let ids = lib.trash_ids()?;
            lib.delete_permanently(&ids, avance)
        },
        root,
    )
}

/// Vaciado automático: elimina lo que lleva 30 días en la Papelera. La
/// interfaz lo pide al abrir la biblioteca y cada hora. Si la fecha del
/// equipo parece adelantada, no borra y avisa con `reloj_adelantado`; la
/// interfaz pregunta y, si el usuario confirma, vuelve a llamar con
/// `confiar_reloj`.
#[tauri::command]
pub fn vaciado_automatico(
    confiar_reloj: bool,
    app: AppHandle,
    estado: State<'_, OpenLibrary>,
) -> Result<(), String> {
    let root = raiz(&estado)?;
    let borrado = app.state::<Borrado>();
    if !empezar(&borrado) {
        // Ya hay otro borrado: el vaciado se hará en la próxima vuelta.
        return Ok(());
    }
    let emisor = app.clone();
    std::thread::spawn(move || {
        let motivo = Motivo::Automatico;
        let evento = match vaciar_caducadas(&emisor, &root, confiar_reloj) {
            Ok(AutoEmpty::Emptied(informe)) => EventoPapelera::terminado(motivo, informe),
            Ok(AutoEmpty::ClockAhead { expired, .. }) => EventoPapelera {
                caducadas: expired,
                ..EventoPapelera::nuevo("reloj_adelantado", motivo)
            },
            Err(mensaje) => EventoPapelera::fallido(motivo, mensaje),
        };
        let _ = emisor.emit("papelera", evento);
        terminar(&emisor.state::<Borrado>());
    });
    Ok(())
}

fn vaciar_caducadas(
    app: &AppHandle,
    root: &Path,
    confiar_reloj: bool,
) -> Result<AutoEmpty, String> {
    let ahora = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;
    let mut lib = Library::open(root).map_err(|e| e.to_string())?;
    lib.empty_expired_trash(ahora, confiar_reloj, |hechas, total| {
        if total > 0 {
            let _ = app.emit(
                "papelera",
                EventoPapelera::en_curso(Motivo::Automatico, hechas, total),
            );
        }
    })
    .map_err(|e| e.to_string())
}

fn raiz(estado: &OpenLibrary) -> Result<PathBuf, String> {
    with_library(estado, |lib| Ok(lib.root().to_path_buf()))
}

/// Marca que empieza un borrado. `false` si ya había otro.
fn empezar(borrado: &Borrado) -> bool {
    let mut activo = lock(&borrado.0);
    if *activo {
        return false;
    }
    *activo = true;
    true
}

fn terminar(borrado: &Borrado) {
    *lock(&borrado.0) = false;
}

/// Lanza `f` en otro hilo con una conexión propia y emite su avance y su
/// resultado como eventos `papelera`.
fn en_segundo_plano(
    app: &AppHandle,
    motivo: Motivo,
    f: impl FnOnce(&mut Library, &mut dyn FnMut(usize, usize)) -> mnemos_core::Result<DeleteReport>
    + Send
    + 'static,
    root: PathBuf,
) -> Result<(), String> {
    if !empezar(&app.state::<Borrado>()) {
        return Err("ya se están eliminando fotos; espera a que termine".to_owned());
    }
    let emisor = app.clone();
    std::thread::spawn(move || {
        let resultado = Library::open(&root).and_then(|mut lib| {
            let mut avance = |hechas: usize, total: usize| {
                let _ = emisor.emit("papelera", EventoPapelera::en_curso(motivo, hechas, total));
            };
            f(&mut lib, &mut avance)
        });
        let evento = match resultado {
            Ok(informe) => EventoPapelera::terminado(motivo, informe),
            Err(err) => EventoPapelera::fallido(motivo, err.to_string()),
        };
        let _ = emisor.emit("papelera", evento);
        terminar(&emisor.state::<Borrado>());
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use mnemos_core::DeleteFailure;

    use super::*;

    #[test]
    fn el_evento_terminado_lleva_las_eliminadas_y_las_fallidas() {
        let informe = DeleteReport {
            deleted: vec![1, 2],
            failed: vec![DeleteFailure {
                id: 3,
                rel_path: "Originales/c.jpg".to_owned(),
                reason: "el archivo ha cambiado".to_owned(),
            }],
        };
        let evento = EventoPapelera::terminado(Motivo::Vaciar, informe);
        let json = serde_json::to_value(&evento).unwrap();
        assert_eq!(json["estado"], "terminado");
        assert_eq!(json["motivo"], "vaciar");
        assert_eq!(json["total"], 3);
        assert_eq!(json["eliminadas"], serde_json::json!([1, 2]));
        assert_eq!(json["fallidas"][0]["ruta"], "Originales/c.jpg");
    }

    #[test]
    fn solo_un_borrado_a_la_vez() {
        let borrado = Borrado::default();
        assert!(empezar(&borrado));
        assert!(!empezar(&borrado));
        terminar(&borrado);
        assert!(empezar(&borrado));
    }
}
