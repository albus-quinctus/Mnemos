//! Acceso directo en el equipo (spec 012): en modo portátil, y solo si el
//! usuario lo pide, MNEMOS se añade al menú de aplicaciones del equipo.
//!
//! En Linux es un `.desktop` en `$XDG_DATA_HOME/applications/` con el icono al
//! lado, en la carpeta de iconos. Lleva la marca [`MARCA`] para que MNEMOS solo
//! toque lo que creó él. Como el punto de montaje del disco puede cambiar,
//! [`AccesoLinux::corregir`] lo reescribe al arrancar si apunta a otra ruta.

// Cada sistema usa solo su parte (Linux o Windows); en macOS, ninguna.
#![allow(dead_code)]

pub mod windows;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;

use crate::Portatil;

/// Línea que marca un `.desktop` como creado por MNEMOS.
pub const MARCA: &str = "X-Mnemos-Portatil=1";

/// Nombre del `.desktop`.
pub const DESKTOP: &str = "mnemos-portatil.desktop";

/// Nombre del icono (sin extensión).
pub const ICONO: &str = "mnemos-portatil";

/// Icono de 256 px de la app.
pub const ICONO_PNG: &[u8] = include_bytes!("../icons/128x128@2x.png");

/// Si el equipo tiene el acceso directo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Estado {
    /// No hay ninguno.
    No,
    /// Hay uno creado por MNEMOS.
    Si,
    /// Hay un archivo con el mismo nombre que no es de MNEMOS: no se toca.
    Ajeno,
}

/// Dónde va el acceso directo en Linux.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccesoLinux {
    /// `…/applications/mnemos-portatil.desktop`
    pub desktop: PathBuf,
    /// `…/icons/hicolor/256x256/apps/mnemos-portatil.png`
    pub icono: PathBuf,
}

impl AccesoLinux {
    /// Rutas a partir de `XDG_DATA_HOME` y `HOME`, como pide la especificación
    /// de directorios de freedesktop: `XDG_DATA_HOME` si es absoluta, y si no
    /// `~/.local/share`.
    pub fn desde(xdg_data_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<Self> {
        let datos = match xdg_data_home.map(Path::new) {
            Some(ruta) if ruta.is_absolute() => ruta.to_owned(),
            _ => {
                let home = Path::new(home?);
                if !home.is_absolute() {
                    return None;
                }
                home.join(".local").join("share")
            }
        };
        Some(Self {
            desktop: datos.join("applications").join(DESKTOP),
            icono: datos
                .join("icons/hicolor/256x256/apps")
                .join(format!("{ICONO}.png")),
        })
    }

    /// Las del usuario que ejecuta la app.
    pub fn del_usuario() -> Option<Self> {
        Self::desde(
            std::env::var_os("XDG_DATA_HOME").as_deref(),
            std::env::var_os("HOME").as_deref(),
        )
    }

    pub fn estado(&self) -> Estado {
        match fs::read_to_string(&self.desktop) {
            Ok(texto) if es_de_mnemos(&texto) => Estado::Si,
            Ok(_) => Estado::Ajeno,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Estado::No,
            // Existe pero no se puede leer (permisos, no es UTF-8): no es nuestro.
            Err(_) => Estado::Ajeno,
        }
    }

    /// Crea el acceso directo a `ejecutable`, o lo reescribe si ya era de
    /// MNEMOS. No sobrescribe un archivo ajeno.
    pub fn crear(&self, ejecutable: &Path) -> Result<(), String> {
        if self.estado() == Estado::Ajeno {
            return Err(format!(
                "Ya hay un {} que no es de MNEMOS; no se toca.",
                self.desktop.display()
            ));
        }
        let texto = contenido_desktop(ejecutable, &self.icono)?;
        escribir(&self.icono, ICONO_PNG)?;
        escribir(&self.desktop, texto.as_bytes())
    }

    /// Quita el acceso directo si es de MNEMOS. Devuelve si había uno.
    pub fn quitar(&self) -> Result<bool, String> {
        match self.estado() {
            Estado::No => Ok(false),
            Estado::Ajeno => Err(format!(
                "{} no es de MNEMOS; no se borra.",
                self.desktop.display()
            )),
            Estado::Si => {
                fs::remove_file(&self.desktop).map_err(|e| error(&self.desktop, &e))?;
                match fs::remove_file(&self.icono) {
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                        Err(error(&self.icono, &e))
                    }
                    _ => Ok(true),
                }
            }
        }
    }

    /// Si hay un acceso directo de MNEMOS que no apunta a `ejecutable` (el
    /// disco se ha montado en otra ruta), lo reescribe. Si no hay ninguno, no
    /// crea nada. Devuelve si lo ha cambiado.
    pub fn corregir(&self, ejecutable: &Path) -> Result<bool, String> {
        if self.estado() != Estado::Si {
            return Ok(false);
        }
        let esperado = contenido_desktop(ejecutable, &self.icono)?;
        let actual = fs::read_to_string(&self.desktop).unwrap_or_default();
        let icono_ok = fs::read(&self.icono).is_ok_and(|png| png == ICONO_PNG);
        if actual == esperado && icono_ok {
            return Ok(false);
        }
        self.crear(ejecutable)?;
        Ok(true)
    }
}

/// Programa que abre el acceso directo en Linux: el `.AppImage` del disco
/// (variable `APPIMAGE`) y no la carpeta temporal donde se monta; sin
/// AppImage, el propio ejecutable.
pub fn ejecutable_linux(exe: &Path, appimage: Option<&Path>) -> PathBuf {
    appimage.unwrap_or(exe).to_owned()
}

/// Qué ve la interfaz: si el equipo admite el acceso directo y si ya existe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Informe {
    pub disponible: bool,
    pub estado: Estado,
}

const NO_DISPONIBLE: &str =
    "El acceso directo solo está disponible en modo portátil, en Linux y Windows.";

/// Si se ofrece el acceso directo: solo en modo portátil (`raiz` es la raíz del
/// disco) y solo en Linux y Windows. En macOS basta con arrastrar la app al Dock.
pub fn disponible(raiz: Option<&Path>) -> bool {
    raiz.is_some() && cfg!(any(target_os = "linux", windows))
}

/// Programa al que apunta el acceso directo.
fn ejecutable() -> Result<PathBuf, String> {
    let exe =
        std::env::current_exe().map_err(|e| format!("No se pudo saber dónde está MNEMOS: {e}"))?;
    if cfg!(target_os = "linux") {
        let appimage = std::env::var_os("APPIMAGE");
        Ok(ejecutable_linux(&exe, appimage.as_deref().map(Path::new)))
    } else {
        Ok(exe)
    }
}

#[cfg(target_os = "linux")]
fn linux() -> Result<AccesoLinux, String> {
    AccesoLinux::del_usuario()
        .ok_or_else(|| "No se encontró la carpeta personal (HOME).".to_owned())
}

#[cfg(target_os = "linux")]
fn estado_del_equipo() -> Result<Estado, String> {
    Ok(linux()?.estado())
}
#[cfg(target_os = "linux")]
fn crear_en_el_equipo(exe: &Path) -> Result<(), String> {
    linux()?.crear(exe)
}
#[cfg(target_os = "linux")]
fn quitar_del_equipo() -> Result<bool, String> {
    linux()?.quitar()
}
#[cfg(target_os = "linux")]
fn corregir_en_el_equipo(exe: &Path) -> Result<bool, String> {
    linux()?.corregir(exe)
}

#[cfg(windows)]
fn estado_del_equipo() -> Result<Estado, String> {
    windows::estado_actual()
}
#[cfg(windows)]
fn crear_en_el_equipo(exe: &Path) -> Result<(), String> {
    windows::crear(exe)
}
#[cfg(windows)]
fn quitar_del_equipo() -> Result<bool, String> {
    windows::quitar()
}
#[cfg(windows)]
fn corregir_en_el_equipo(exe: &Path) -> Result<bool, String> {
    windows::corregir(exe)
}

#[cfg(not(any(target_os = "linux", windows)))]
fn estado_del_equipo() -> Result<Estado, String> {
    Err(NO_DISPONIBLE.to_owned())
}
#[cfg(not(any(target_os = "linux", windows)))]
fn crear_en_el_equipo(_exe: &Path) -> Result<(), String> {
    Err(NO_DISPONIBLE.to_owned())
}
#[cfg(not(any(target_os = "linux", windows)))]
fn quitar_del_equipo() -> Result<bool, String> {
    Err(NO_DISPONIBLE.to_owned())
}
#[cfg(not(any(target_os = "linux", windows)))]
fn corregir_en_el_equipo(_exe: &Path) -> Result<bool, String> {
    Err(NO_DISPONIBLE.to_owned())
}

fn exigir_disponible(portatil: &Portatil) -> Result<(), String> {
    if disponible(portatil.0.as_deref()) {
        Ok(())
    } else {
        Err(NO_DISPONIBLE.to_owned())
    }
}

/// ¿Se ofrece el acceso directo en este equipo y ya existe? Los comandos son
/// `async` para que PowerShell, en Windows, no congele la interfaz.
#[tauri::command(async)]
pub fn acceso_directo_estado(portatil: State<'_, Portatil>) -> Result<Informe, String> {
    if !disponible(portatil.0.as_deref()) {
        return Ok(Informe {
            disponible: false,
            estado: Estado::No,
        });
    }
    Ok(Informe {
        disponible: true,
        estado: estado_del_equipo()?,
    })
}

/// Crea el acceso directo en este equipo (o lo actualiza si ya era de MNEMOS).
#[tauri::command(async)]
pub fn crear_acceso_directo(portatil: State<'_, Portatil>) -> Result<Informe, String> {
    exigir_disponible(&portatil)?;
    crear_en_el_equipo(&ejecutable()?)?;
    Ok(Informe {
        disponible: true,
        estado: estado_del_equipo()?,
    })
}

/// Quita el acceso directo de este equipo. Solo borra lo que creó MNEMOS.
#[tauri::command(async)]
pub fn quitar_acceso_directo(portatil: State<'_, Portatil>) -> Result<Informe, String> {
    exigir_disponible(&portatil)?;
    quitar_del_equipo()?;
    Ok(Informe {
        disponible: true,
        estado: estado_del_equipo()?,
    })
}

/// Al arrancar en modo portátil, reescribe el acceso directo si apunta a otra
/// ruta (el disco se montó en otro sitio). En segundo plano y sin molestar:
/// si falla, solo lo escribe en el registro de errores.
pub fn corregir_al_arrancar(raiz: Option<&Path>) {
    if !disponible(raiz) {
        return;
    }
    std::thread::spawn(|| {
        if let Err(e) = ejecutable().and_then(|exe| corregir_en_el_equipo(&exe)) {
            eprintln!("acceso directo: {e}");
        }
    });
}

fn es_de_mnemos(texto: &str) -> bool {
    texto.lines().any(|linea| linea.trim_end() == MARCA)
}

/// Contenido del `.desktop`. Falla si una ruta no se puede escribir en él
/// (no es UTF-8 o lleva saltos de línea).
pub fn contenido_desktop(ejecutable: &Path, icono: &Path) -> Result<String, String> {
    let exe = texto_de(ejecutable)?;
    let icono = texto_de(icono)?;
    Ok(format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.5\n\
         Name=MNEMOS\n\
         GenericName=Gestor de fotos\n\
         Comment=Tus fotos, en tu disco\n\
         Exec={}\n\
         TryExec={}\n\
         Icon={}\n\
         Terminal=false\n\
         Categories=Graphics;Photography;\n\
         {MARCA}\n",
        exec(exe),
        cadena(exe),
        cadena(icono),
    ))
}

fn texto_de(ruta: &Path) -> Result<&str, String> {
    let texto = ruta.to_str().ok_or_else(|| {
        format!(
            "La ruta {} tiene caracteres que no se pueden usar en un acceso directo.",
            ruta.display()
        )
    })?;
    if texto.contains(['\n', '\r']) {
        return Err(format!(
            "La ruta {} lleva saltos de línea; no se puede usar en un acceso directo.",
            ruta.display()
        ));
    }
    Ok(texto)
}

/// Valor de tipo cadena: la barra invertida se escribe `\\`.
fn cadena(texto: &str) -> String {
    texto.replace('\\', "\\\\")
}

/// Valor de `Exec` con un solo argumento: entre comillas dobles, con `\`
/// delante de `"`, `` ` ``, `$` y `\`; `%` se escribe `%%`; y después, la
/// regla de las cadenas (así una barra invertida acaba siendo `\\\\`).
fn exec(ruta: &str) -> String {
    let mut citado = String::with_capacity(ruta.len() + 2);
    citado.push('"');
    for c in ruta.chars() {
        match c {
            '"' | '`' | '$' | '\\' => {
                citado.push('\\');
                citado.push(c);
            }
            '%' => citado.push_str("%%"),
            _ => citado.push(c),
        }
    }
    citado.push('"');
    cadena(&citado)
}

/// Escribe `datos` en `ruta` creando la carpeta, primero en un temporal para
/// no dejar el archivo a medias.
fn escribir(ruta: &Path, datos: &[u8]) -> Result<(), String> {
    let carpeta = ruta
        .parent()
        .ok_or_else(|| format!("{} no tiene carpeta.", ruta.display()))?;
    fs::create_dir_all(carpeta).map_err(|e| error(carpeta, &e))?;
    let mut temporal = ruta.as_os_str().to_owned();
    temporal.push(".tmp");
    let temporal = PathBuf::from(temporal);
    fs::write(&temporal, datos).map_err(|e| error(&temporal, &e))?;
    fs::rename(&temporal, ruta).map_err(|e| {
        let _ = fs::remove_file(&temporal);
        error(ruta, &e)
    })
}

fn error(ruta: &Path, e: &std::io::Error) -> String {
    format!("No se pudo escribir en {}: {e}", ruta.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acceso(dir: &Path) -> AccesoLinux {
        AccesoLinux::desde(Some(dir.as_os_str()), None).unwrap()
    }

    /// Lee el valor de `clave=` del `.desktop`.
    fn valor<'a>(texto: &'a str, clave: &str) -> &'a str {
        texto
            .lines()
            .find_map(|l| l.strip_prefix(clave)?.strip_prefix('='))
            .unwrap()
    }

    #[test]
    fn rutas_segun_xdg_y_home() {
        let a = AccesoLinux::desde(None, Some(OsStr::new("/home/lilith"))).unwrap();
        assert_eq!(
            a.desktop,
            Path::new("/home/lilith/.local/share/applications/mnemos-portatil.desktop")
        );
        assert_eq!(
            a.icono,
            Path::new("/home/lilith/.local/share/icons/hicolor/256x256/apps/mnemos-portatil.png")
        );
        let a =
            AccesoLinux::desde(Some(OsStr::new("/datos")), Some(OsStr::new("/home/u"))).unwrap();
        assert_eq!(
            a.desktop,
            Path::new("/datos/applications/mnemos-portatil.desktop")
        );
        // Una XDG_DATA_HOME relativa no vale: se usa HOME.
        let a = AccesoLinux::desde(Some(OsStr::new("datos")), Some(OsStr::new("/home/u"))).unwrap();
        assert!(a.desktop.starts_with("/home/u/.local/share"));
        assert_eq!(AccesoLinux::desde(None, None), None);
        assert_eq!(AccesoLinux::desde(None, Some(OsStr::new("relativa"))), None);
    }

    #[test]
    fn el_appimage_manda_sobre_el_ejecutable_montado() {
        let montado = Path::new("/tmp/.mount_MNEMOS/usr/bin/mnemos-desktop");
        let appimage = Path::new("/media/lilith/FOTOS/MNEMOS-linux.AppImage");
        assert_eq!(ejecutable_linux(montado, Some(appimage)), appimage);
        assert_eq!(ejecutable_linux(montado, None), montado);
    }

    #[test]
    fn solo_se_ofrece_en_modo_portatil_y_en_linux_o_windows() {
        assert!(!disponible(None));
        assert_eq!(
            disponible(Some(Path::new("/media/u/DISCO"))),
            cfg!(any(target_os = "linux", windows))
        );
    }

    #[test]
    fn el_informe_viaja_a_la_interfaz_en_minusculas() {
        let informe = Informe {
            disponible: true,
            estado: Estado::Si,
        };
        assert_eq!(
            serde_json::to_string(&informe).unwrap(),
            r#"{"disponible":true,"estado":"si"}"#
        );
        assert_eq!(serde_json::to_string(&Estado::Ajeno).unwrap(), r#""ajeno""#);
        assert_eq!(serde_json::to_string(&Estado::No).unwrap(), r#""no""#);
    }

    #[test]
    fn exec_escapa_espacios_tildes_comillas_y_porcentajes() {
        assert_eq!(
            exec("/media/lilith/FOTOS/MNEMOS-linux.AppImage"),
            r#""/media/lilith/FOTOS/MNEMOS-linux.AppImage""#
        );
        assert_eq!(
            exec("/media/u/Mis fotos/Año 100%/MNEMOS.AppImage"),
            r#""/media/u/Mis fotos/Año 100%%/MNEMOS.AppImage""#
        );
        assert_eq!(exec(r#"/m/a"b`c$d"#), r#""/m/a\\"b\\`c\\$d""#);
        // Una barra invertida acaba siendo cuatro.
        assert_eq!(exec(r"/m/a\b"), r#""/m/a\\\\b""#);
    }

    #[test]
    fn el_desktop_lleva_la_ruta_el_icono_y_la_marca() {
        let texto = contenido_desktop(
            Path::new("/media/u/Mis fotos/MNEMOS-linux.AppImage"),
            Path::new("/home/u/.local/share/icons/hicolor/256x256/apps/mnemos-portatil.png"),
        )
        .unwrap();
        assert!(texto.starts_with("[Desktop Entry]\n"));
        assert_eq!(
            valor(&texto, "Exec"),
            r#""/media/u/Mis fotos/MNEMOS-linux.AppImage""#
        );
        assert_eq!(
            valor(&texto, "TryExec"),
            "/media/u/Mis fotos/MNEMOS-linux.AppImage"
        );
        assert_eq!(valor(&texto, "Type"), "Application");
        assert!(es_de_mnemos(&texto));
    }

    #[test]
    fn rutas_con_salto_de_linea_no_se_aceptan() {
        assert!(contenido_desktop(Path::new("/m/a\nExec=rm"), Path::new("/i.png")).is_err());
    }

    #[test]
    fn crea_y_quita_el_acceso_directo() {
        let dir = tempfile::tempdir().unwrap();
        let a = acceso(dir.path());
        assert_eq!(a.estado(), Estado::No);
        a.crear(Path::new("/media/u/FOTOS/MNEMOS-linux.AppImage"))
            .unwrap();
        assert_eq!(a.estado(), Estado::Si);
        assert_eq!(fs::read(&a.icono).unwrap(), ICONO_PNG);
        assert!(a.quitar().unwrap());
        assert_eq!(a.estado(), Estado::No);
        assert!(!a.icono.exists());
        assert!(!a.quitar().unwrap());
    }

    #[test]
    fn no_toca_un_desktop_ajeno() {
        let dir = tempfile::tempdir().unwrap();
        let a = acceso(dir.path());
        fs::create_dir_all(a.desktop.parent().unwrap()).unwrap();
        let ajeno = "[Desktop Entry]\nName=Otro\nExec=otro\n";
        fs::write(&a.desktop, ajeno).unwrap();
        assert_eq!(a.estado(), Estado::Ajeno);
        assert!(a.crear(Path::new("/m/MNEMOS.AppImage")).is_err());
        assert!(a.quitar().is_err());
        assert!(!a.corregir(Path::new("/m/MNEMOS.AppImage")).unwrap());
        assert_eq!(fs::read_to_string(&a.desktop).unwrap(), ajeno);
    }

    #[test]
    fn se_corrige_si_cambia_el_punto_de_montaje() {
        let dir = tempfile::tempdir().unwrap();
        let a = acceso(dir.path());
        let antes = Path::new("/media/lilith/FOTOS/MNEMOS-linux.AppImage");
        let despues = Path::new("/run/media/paulo/FOTOS/MNEMOS-linux.AppImage");
        a.crear(antes).unwrap();
        assert!(!a.corregir(antes).unwrap());
        assert!(a.corregir(despues).unwrap());
        let texto = fs::read_to_string(&a.desktop).unwrap();
        assert_eq!(valor(&texto, "TryExec"), despues.to_str().unwrap());
        // Si falta el icono, también lo repone.
        fs::remove_file(&a.icono).unwrap();
        assert!(a.corregir(despues).unwrap());
        assert!(a.icono.is_file());
    }

    #[test]
    fn corregir_no_crea_nada_si_no_habia_acceso_directo() {
        let dir = tempfile::tempdir().unwrap();
        let a = acceso(dir.path());
        assert!(!a.corregir(Path::new("/m/MNEMOS.AppImage")).unwrap());
        assert!(!a.desktop.exists());
        assert!(!a.icono.exists());
    }
}
