//! Configuración de la aplicación: la última biblioteca abierta (spec 002) y
//! el tema de la interfaz (spec 013).
//!
//! Se guarda en la carpeta de configuración del sistema (o en el disco, en
//! modo portátil) y no en la biblioteca: son preferencias de la aplicación, y
//! la ruta es absoluta y cambia de un equipo a otro.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize};

const ARCHIVO: &str = "configuracion.json";

/// Tema de la interfaz elegido en Ajustes (spec 013).
#[derive(Debug, Default, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tema {
    /// Sigue la preferencia del sistema operativo.
    #[default]
    Sistema,
    Claro,
    Oscuro,
}

/// Un valor desconocido (de una versión futura, o escrito a mano) se lee como
/// `Sistema` en vez de invalidar toda la configuración.
impl<'de> Deserialize<'de> for Tema {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match String::deserialize(d)?.as_str() {
            "claro" => Tema::Claro,
            "oscuro" => Tema::Oscuro,
            _ => Tema::Sistema,
        })
    }
}

impl Tema {
    /// Tema nativo de la ventana (barra de título, menús del sistema).
    /// `None` deja que mande el sistema.
    pub fn nativo(self) -> Option<tauri::Theme> {
        match self {
            Tema::Sistema => None,
            Tema::Claro => Some(tauri::Theme::Light),
            Tema::Oscuro => Some(tauri::Theme::Dark),
        }
    }

    /// Guion que se ejecuta antes de cargar la página: pone `data-tema` en
    /// `<html>` para que no haya destello del tema contrario al arrancar. Si
    /// `<html>` aún no existe, espera a que aparezca.
    pub fn guion_inicial(self) -> Option<String> {
        let valor = match self {
            Tema::Sistema => return None,
            Tema::Claro => "claro",
            Tema::Oscuro => "oscuro",
        };
        Some(format!(
            "(function (t) {{\n\
             var poner = function () {{ document.documentElement.dataset.tema = t; }};\n\
             if (document.documentElement) {{ poner(); return; }}\n\
             new MutationObserver(function (_, o) {{\n\
             if (document.documentElement) {{ poner(); o.disconnect(); }}\n\
             }}).observe(document, {{ childList: true }});\n\
             }})(\"{valor}\");"
        ))
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Configuracion {
    pub ultima_biblioteca: Option<PathBuf>,
    #[serde(default)]
    pub tema: Tema,
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

    /// Lee la configuración de `carpeta`, aplica `cambio` y la guarda: cambiar
    /// un campo (la última biblioteca, el tema) no borra los demás.
    pub fn cambiar(carpeta: &Path, cambio: impl FnOnce(&mut Self)) -> io::Result<()> {
        let mut config = Self::leer(carpeta);
        cambio(&mut config);
        config.guardar(carpeta)
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
            ..Default::default()
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

    #[test]
    fn una_configuracion_antigua_sin_tema_sigue_al_sistema() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(ARCHIVO),
            r#"{ "ultima_biblioteca": "/media/disco/Biblioteca" }"#,
        )
        .unwrap();
        let config = Configuracion::leer(dir.path());
        assert_eq!(config.tema, Tema::Sistema);
        assert_eq!(
            config.ultima_biblioteca,
            Some(PathBuf::from("/media/disco/Biblioteca"))
        );
    }

    #[test]
    fn un_tema_desconocido_sigue_al_sistema_sin_perder_lo_demas() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(ARCHIVO),
            r#"{ "ultima_biblioteca": "MNEMOS", "tema": "sepia" }"#,
        )
        .unwrap();
        let config = Configuracion::leer(dir.path());
        assert_eq!(config.tema, Tema::Sistema);
        assert_eq!(config.ultima_biblioteca, Some(PathBuf::from("MNEMOS")));
    }

    #[test]
    fn recuerda_el_tema() {
        let dir = tempfile::tempdir().unwrap();
        for tema in [Tema::Claro, Tema::Oscuro, Tema::Sistema] {
            let config = Configuracion {
                tema,
                ..Default::default()
            };
            config.guardar(dir.path()).unwrap();
            assert_eq!(Configuracion::leer(dir.path()).tema, tema);
        }
        let texto = fs::read_to_string(dir.path().join(ARCHIVO)).unwrap();
        assert!(texto.contains(r#""tema": "sistema""#), "{texto}");
    }

    #[test]
    fn el_guion_inicial_solo_existe_si_se_fuerza_un_tema() {
        assert_eq!(Tema::Sistema.guion_inicial(), None);
        assert_eq!(Tema::Sistema.nativo(), None);
        let guion = Tema::Oscuro.guion_inicial().unwrap();
        assert!(guion.contains(r#"("oscuro")"#), "{guion}");
        assert!(guion.contains("MutationObserver"));
        assert_eq!(Tema::Claro.nativo(), Some(tauri::Theme::Light));
    }

    #[test]
    fn cambiar_un_campo_conserva_los_demas() {
        let dir = tempfile::tempdir().unwrap();
        let biblioteca = PathBuf::from("/media/disco/Biblioteca");
        Configuracion::cambiar(dir.path(), |c| {
            c.ultima_biblioteca = Some(biblioteca.clone())
        })
        .unwrap();
        Configuracion::cambiar(dir.path(), |c| c.tema = Tema::Oscuro).unwrap();
        assert_eq!(
            Configuracion::leer(dir.path()),
            Configuracion {
                ultima_biblioteca: Some(biblioteca.clone()),
                tema: Tema::Oscuro,
            }
        );
        Configuracion::cambiar(dir.path(), |c| c.ultima_biblioteca = None).unwrap();
        assert_eq!(Configuracion::leer(dir.path()).tema, Tema::Oscuro);
    }
}
