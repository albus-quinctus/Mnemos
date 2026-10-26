//! Configuración de la aplicación: la última biblioteca abierta (spec 002).
//!
//! Se guarda en la carpeta de configuración del sistema y no en la biblioteca,
//! porque la ruta es absoluta y cambia de un equipo a otro.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const ARCHIVO: &str = "configuracion.json";

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Configuracion {
    pub ultima_biblioteca: Option<PathBuf>,
}

impl Configuracion {
    /// Lee la configuración de `carpeta`. Si falta o está dañada, devuelve la
    /// configuración vacía: nunca impide abrir la aplicación.
    pub fn leer(carpeta: &Path) -> Self {
        fs::read_to_string(carpeta.join(ARCHIVO))
            .ok()
            .and_then(|texto| serde_json::from_str(&texto).ok())
            .unwrap_or_default()
    }

    /// Guarda la configuración en `carpeta`, creándola si hace falta. Primero
    /// se escribe a un archivo temporal, así que un corte no deja el archivo a medias.
    pub fn guardar(&self, carpeta: &Path) -> io::Result<()> {
        fs::create_dir_all(carpeta)?;
        let texto = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        let temporal = carpeta.join(format!("{ARCHIVO}.tmp"));
        fs::write(&temporal, texto)?;
        fs::rename(temporal, carpeta.join(ARCHIVO))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_archivo_la_configuracion_esta_vacia() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Configuracion::leer(dir.path()), Configuracion::default());
    }

    #[test]
    fn recuerda_la_ultima_biblioteca() {
        let dir = tempfile::tempdir().unwrap();
        let config = Configuracion {
            ultima_biblioteca: Some(PathBuf::from("/media/disco/Biblioteca")),
        };
        config.guardar(&dir.path().join("mnemos")).unwrap();
        assert_eq!(Configuracion::leer(&dir.path().join("mnemos")), config);
        assert!(
            !dir.path()
                .join("mnemos")
                .join("configuracion.json.tmp")
                .exists()
        );
    }

    #[test]
    fn un_archivo_danado_no_impide_arrancar() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(ARCHIVO), "{ no es json").unwrap();
        assert_eq!(Configuracion::leer(dir.path()), Configuracion::default());
    }
}
