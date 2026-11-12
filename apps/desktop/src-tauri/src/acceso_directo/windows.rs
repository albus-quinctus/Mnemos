//! Acceso directo de Windows (spec 012): `MNEMOS.lnk` en el menú Inicio y en el
//! escritorio del usuario.
//!
//! Los `.lnk` se leen y se escriben con PowerShell (`WScript.Shell`), que viene
//! con Windows 10 y 11, y así no hace falta ninguna dependencia nueva. El
//! script es siempre el mismo texto fijo: las rutas le llegan por variables de
//! entorno y nunca se pegan dentro del script, así que una ruta con comillas,
//! `$` o tildes no puede cambiar lo que se ejecuta. PowerShell también resuelve
//! las carpetas Inicio y Escritorio, que a veces están redirigidas a OneDrive.
//!
//! La descripción del `.lnk` lleva [`MARCA`] para que MNEMOS solo borre o
//! reescriba los que creó él.

use std::path::{Path, PathBuf};

use super::Estado;

/// Texto de la descripción que marca un `.lnk` como creado por MNEMOS.
pub const MARCA: &str = "X-Mnemos-Portatil=1";

/// Nombre del acceso directo.
pub const NOMBRE: &str = "MNEMOS.lnk";

/// Separador de las rutas en la variable de entorno: Windows no admite `|` en
/// un nombre de archivo.
const SEPARADOR: char = '|';

/// Lista las carpetas de destino, una ruta por línea.
const SCRIPT_CARPETAS: &str = "\
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false
foreach ($c in 'Programs', 'Desktop') {
    $d = [Environment]::GetFolderPath($c)
    if ($d) { [Console]::Out.WriteLine((Join-Path $d 'MNEMOS.lnk')) }
}";

/// Por cada ruta de `MNEMOS_RUTAS`, una línea `ruta<TAB>descripción<TAB>destino`
/// si el `.lnk` existe, o `ruta` sola si no existe.
const SCRIPT_LEER: &str = "\
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false
$sh = New-Object -ComObject WScript.Shell
foreach ($r in $env:MNEMOS_RUTAS.Split('|')) {
    if (Test-Path -LiteralPath $r -PathType Leaf) {
        $l = $sh.CreateShortcut($r)
        $d = ($l.Description -replace '[\\t\\r\\n]', ' ')
        [Console]::Out.WriteLine($r + \"`t\" + $d + \"`t\" + $l.TargetPath)
    } else {
        [Console]::Out.WriteLine($r)
    }
}";

/// Crea o reescribe cada `.lnk` de `MNEMOS_RUTAS` apuntando a `MNEMOS_EXE`,
/// con el icono del propio ejecutable y la carpeta del disco como carpeta de
/// trabajo.
const SCRIPT_ESCRIBIR: &str = "\
$ErrorActionPreference = 'Stop'
$sh = New-Object -ComObject WScript.Shell
$exe = $env:MNEMOS_EXE
foreach ($r in $env:MNEMOS_RUTAS.Split('|')) {
    $l = $sh.CreateShortcut($r)
    $l.TargetPath = $exe
    $l.WorkingDirectory = Split-Path -Parent $exe
    $l.IconLocation = $exe + ',0'
    $l.Description = $env:MNEMOS_DESCRIPCION
    $l.Save()
}";

/// Un `.lnk` que existe: su descripción y adónde apunta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Existente {
    pub ruta: PathBuf,
    pub descripcion: String,
    pub destino: String,
}

/// Lo que hay en una de las carpetas de destino.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destino {
    pub ruta: PathBuf,
    pub existente: Option<Existente>,
}

impl Destino {
    pub fn estado(&self) -> Estado {
        match &self.existente {
            None => Estado::No,
            Some(e) if es_de_mnemos(&e.descripcion) => Estado::Si,
            Some(_) => Estado::Ajeno,
        }
    }
}

fn es_de_mnemos(descripcion: &str) -> bool {
    descripcion.contains(MARCA)
}

/// Descripción que se escribe en el `.lnk`.
pub fn descripcion() -> String {
    format!("MNEMOS, tus fotos en tu disco · {MARCA}")
}

/// Estado de conjunto: `Ajeno` si algún destino es de otro programa (manda
/// sobre todo, porque impide crear), `Si` si hay alguno nuestro y `No` si no
/// hay ninguno.
pub fn estado(destinos: &[Destino]) -> Estado {
    let mut resultado = Estado::No;
    for d in destinos {
        match d.estado() {
            Estado::Ajeno => return Estado::Ajeno,
            Estado::Si => resultado = Estado::Si,
            Estado::No => {}
        }
    }
    resultado
}

/// Interpreta la salida de [`SCRIPT_LEER`].
pub fn interpretar(salida: &str) -> Vec<Destino> {
    salida
        .trim_start_matches('\u{feff}')
        .lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.is_empty())
        .map(|linea| {
            let mut partes = linea.splitn(3, '\t');
            let ruta = PathBuf::from(partes.next().unwrap_or_default());
            let existente = match (partes.next(), partes.next()) {
                (Some(descripcion), Some(destino)) => Some(Existente {
                    ruta: ruta.clone(),
                    descripcion: descripcion.to_owned(),
                    destino: destino.to_owned(),
                }),
                _ => None,
            };
            Destino { ruta, existente }
        })
        .collect()
}

/// Une las rutas para la variable de entorno. Falla si alguna tiene el
/// separador, que Windows no permite en un nombre de archivo.
fn unir(rutas: &[&Path]) -> Result<String, String> {
    let mut textos = Vec::with_capacity(rutas.len());
    for ruta in rutas {
        let texto = ruta.to_str().ok_or_else(|| {
            format!(
                "La ruta {} tiene caracteres que no se pueden usar en un acceso directo.",
                ruta.display()
            )
        })?;
        if texto.contains([SEPARADOR, '\n', '\r']) {
            return Err(format!(
                "La ruta {texto} no se puede usar en un acceso directo."
            ));
        }
        textos.push(texto);
    }
    Ok(textos.join(&SEPARADOR.to_string()))
}

/// Ejecuta un script de PowerShell sin ventana y devuelve su salida.
#[cfg(windows)]
fn powershell(script: &str, entorno: &[(&str, &str)]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // CREATE_NO_WINDOW: que no parpadee una consola al pulsar el botón.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut orden = Command::new("powershell.exe");
    orden
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
        ])
        .args(["-Command", "-"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    for (clave, valor) in entorno {
        orden.env(clave, valor);
    }
    let mut hijo = orden
        .spawn()
        .map_err(|e| format!("No se pudo ejecutar PowerShell: {e}"))?;
    // El script entra por la entrada estándar y no por la línea de órdenes,
    // para no pelear con el entrecomillado de Windows.
    if let Some(mut entrada) = hijo.stdin.take() {
        use std::io::Write;
        entrada
            .write_all(script.as_bytes())
            .map_err(|e| format!("No se pudo ejecutar PowerShell: {e}"))?;
    }
    let salida = hijo
        .wait_with_output()
        .map_err(|e| format!("No se pudo ejecutar PowerShell: {e}"))?;
    if !salida.status.success() {
        return Err(format!(
            "PowerShell falló: {}",
            String::from_utf8_lossy(&salida.stderr).trim()
        ));
    }
    // Los scripts fijan la salida en UTF-8 (sin BOM) para que las rutas con
    // tildes no se estropeen con la página de códigos de la consola.
    Ok(String::from_utf8_lossy(&salida.stdout).into_owned())
}

#[cfg(not(windows))]
fn powershell(_script: &str, _entorno: &[(&str, &str)]) -> Result<String, String> {
    Err("Los accesos directos de Windows solo se pueden crear en Windows.".to_owned())
}

/// Dónde irían los `.lnk` en este equipo.
fn carpetas() -> Result<Vec<PathBuf>, String> {
    let salida = powershell(SCRIPT_CARPETAS, &[])?;
    let rutas: Vec<PathBuf> = salida
        .trim_start_matches('\u{feff}')
        .lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect();
    if rutas.is_empty() {
        return Err("No se encontró el menú Inicio ni el escritorio.".to_owned());
    }
    Ok(rutas)
}

/// Lo que hay hoy en cada carpeta de destino.
pub fn destinos() -> Result<Vec<Destino>, String> {
    let rutas = carpetas()?;
    let refs: Vec<&Path> = rutas.iter().map(PathBuf::as_path).collect();
    let salida = powershell(SCRIPT_LEER, &[("MNEMOS_RUTAS", &unir(&refs)?)])?;
    Ok(interpretar(&salida))
}

fn escribir(rutas: &[&Path], ejecutable: &Path) -> Result<(), String> {
    let exe = ejecutable.to_str().ok_or_else(|| {
        format!(
            "La ruta {} tiene caracteres que no se pueden usar en un acceso directo.",
            ejecutable.display()
        )
    })?;
    let descripcion = descripcion();
    powershell(
        SCRIPT_ESCRIBIR,
        &[
            ("MNEMOS_RUTAS", &unir(rutas)?),
            ("MNEMOS_EXE", exe),
            ("MNEMOS_DESCRIPCION", &descripcion),
        ],
    )
    .map(drop)
}

/// Estado del acceso directo en este equipo.
pub fn estado_actual() -> Result<Estado, String> {
    destinos().map(|d| estado(&d))
}

/// Crea los `.lnk` apuntando a `ejecutable`, o los reescribe si ya eran de
/// MNEMOS. Si alguno es de otro programa no escribe ninguno.
pub fn crear(ejecutable: &Path) -> Result<(), String> {
    let destinos = destinos()?;
    if let Some(ajeno) = destinos.iter().find(|d| d.estado() == Estado::Ajeno) {
        return Err(format!(
            "Ya hay un {} que no es de MNEMOS; no se toca.",
            ajeno.ruta.display()
        ));
    }
    let rutas: Vec<&Path> = destinos.iter().map(|d| d.ruta.as_path()).collect();
    escribir(&rutas, ejecutable)
}

/// Quita los `.lnk` de MNEMOS. Devuelve si había alguno. Un `.lnk` ajeno no
/// se borra y da error.
pub fn quitar() -> Result<bool, String> {
    let destinos = destinos()?;
    if let Some(ajeno) = destinos.iter().find(|d| d.estado() == Estado::Ajeno) {
        return Err(format!(
            "{} no es de MNEMOS; no se borra.",
            ajeno.ruta.display()
        ));
    }
    let mut habia = false;
    for d in destinos.iter().filter(|d| d.estado() == Estado::Si) {
        std::fs::remove_file(&d.ruta)
            .map_err(|e| format!("No se pudo borrar {}: {e}", d.ruta.display()))?;
        habia = true;
    }
    Ok(habia)
}

/// Qué `.lnk` de MNEMOS no apuntan a `ejecutable`. Solo mira los que ya
/// existen y son nuestros: no crea ninguno.
pub fn desactualizados<'a>(destinos: &'a [Destino], ejecutable: &Path) -> Vec<&'a Path> {
    let esperado = ejecutable.to_string_lossy();
    destinos
        .iter()
        .filter(|d| d.estado() == Estado::Si)
        .filter(|d| {
            d.existente
                .as_ref()
                .is_some_and(|e| !mismo_destino(&e.destino, &esperado))
        })
        .map(|d| d.ruta.as_path())
        .collect()
}

/// Windows no distingue mayúsculas en las rutas ni `/` de `\`.
fn mismo_destino(a: &str, b: &str) -> bool {
    let normal = |s: &str| s.replace('/', "\\").to_lowercase();
    normal(a) == normal(b)
}

/// Reescribe los `.lnk` de MNEMOS que apuntan a otra ruta (el disco ha
/// cambiado de letra). Si no hay ninguno, no crea nada. Devuelve si cambió algo.
pub fn corregir(ejecutable: &Path) -> Result<bool, String> {
    let destinos = destinos()?;
    let a_corregir = desactualizados(&destinos, ejecutable);
    if a_corregir.is_empty() {
        return Ok(false);
    }
    escribir(&a_corregir, ejecutable)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nuestro(ruta: &str, destino: &str) -> Destino {
        Destino {
            ruta: PathBuf::from(ruta),
            existente: Some(Existente {
                ruta: PathBuf::from(ruta),
                descripcion: descripcion(),
                destino: destino.to_owned(),
            }),
        }
    }

    fn ajeno(ruta: &str) -> Destino {
        Destino {
            ruta: PathBuf::from(ruta),
            existente: Some(Existente {
                ruta: PathBuf::from(ruta),
                descripcion: "Otro programa".to_owned(),
                destino: r"C:\Otro\otro.exe".to_owned(),
            }),
        }
    }

    fn vacio(ruta: &str) -> Destino {
        Destino {
            ruta: PathBuf::from(ruta),
            existente: None,
        }
    }

    #[test]
    fn interpreta_la_salida_de_powershell() {
        let salida = "C:\\Users\\Ana\\Start\\MNEMOS.lnk\tMNEMOS · X-Mnemos-Portatil=1\tE:\\MNEMOS.exe\r\n\
                      C:\\Users\\Ana\\Desktop\\MNEMOS.lnk\r\n";
        let destinos = interpretar(salida);
        assert_eq!(destinos.len(), 2);
        assert_eq!(destinos[0].estado(), Estado::Si);
        assert_eq!(
            destinos[0].existente.as_ref().unwrap().destino,
            r"E:\MNEMOS.exe"
        );
        assert_eq!(destinos[1].estado(), Estado::No);
        assert!(interpretar("").is_empty());
    }

    #[test]
    fn la_marca_distingue_lo_nuestro_de_lo_ajeno() {
        assert_eq!(nuestro("a.lnk", "x").estado(), Estado::Si);
        assert_eq!(ajeno("a.lnk").estado(), Estado::Ajeno);
        assert_eq!(vacio("a.lnk").estado(), Estado::No);
    }

    #[test]
    fn un_destino_ajeno_manda_sobre_el_estado_de_conjunto() {
        assert_eq!(estado(&[vacio("a"), vacio("b")]), Estado::No);
        assert_eq!(estado(&[nuestro("a", "x"), vacio("b")]), Estado::Si);
        assert_eq!(estado(&[nuestro("a", "x"), ajeno("b")]), Estado::Ajeno);
        assert_eq!(estado(&[]), Estado::No);
    }

    #[test]
    fn detecta_los_que_apuntan_a_otra_letra_de_unidad() {
        let destinos = [
            nuestro(r"C:\Inicio\MNEMOS.lnk", r"E:\MNEMOS.exe"),
            nuestro(r"C:\Escritorio\MNEMOS.lnk", r"F:\MNEMOS.exe"),
            ajeno(r"C:\Otro\MNEMOS.lnk"),
            vacio(r"C:\Nada\MNEMOS.lnk"),
        ];
        let viejos = desactualizados(&destinos, Path::new(r"F:\MNEMOS.exe"));
        assert_eq!(viejos, [Path::new(r"C:\Inicio\MNEMOS.lnk")]);
        // Mayúsculas y barras no cuentan como cambio.
        assert!(desactualizados(&destinos[1..2], Path::new("f:/mnemos.EXE")).is_empty());
        // Sin ningún .lnk nuestro no hay nada que corregir.
        assert!(desactualizados(&[vacio("a")], Path::new(r"F:\MNEMOS.exe")).is_empty());
    }

    #[test]
    fn las_rutas_se_unen_sin_peligro() {
        let uno = Path::new(r"C:\Users\Ana\Menú Inicio\MNEMOS.lnk");
        let dos = Path::new(r"C:\Users\Ana\Escritorio\MNEMOS.lnk");
        assert_eq!(
            unir(&[uno, dos]).unwrap(),
            r"C:\Users\Ana\Menú Inicio\MNEMOS.lnk|C:\Users\Ana\Escritorio\MNEMOS.lnk"
        );
        assert!(unir(&[Path::new("C:\\a|b\\MNEMOS.lnk")]).is_err());
        assert!(unir(&[Path::new("C:\\a\nb")]).is_err());
    }

    #[test]
    fn los_scripts_no_llevan_rutas_pegadas() {
        // Las rutas solo entran por variables de entorno.
        for script in [SCRIPT_CARPETAS, SCRIPT_LEER, SCRIPT_ESCRIBIR] {
            assert!(!script.contains("{}"));
        }
        assert!(SCRIPT_LEER.contains("$env:MNEMOS_RUTAS"));
        assert!(SCRIPT_ESCRIBIR.contains("$env:MNEMOS_EXE"));
    }

    #[test]
    fn los_scripts_son_ascii_y_fijan_la_salida_en_utf8() {
        // Entran por la entrada estándar con la página de códigos de la
        // consola: si llevaran tildes, se estropearían.
        for script in [SCRIPT_CARPETAS, SCRIPT_LEER, SCRIPT_ESCRIBIR] {
            assert!(script.is_ascii());
        }
        for script in [SCRIPT_CARPETAS, SCRIPT_LEER] {
            assert!(script.contains("[Console]::OutputEncoding"));
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn fuera_de_windows_no_se_puede() {
        assert!(estado_actual().is_err());
    }
}
