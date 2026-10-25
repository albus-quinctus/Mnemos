use std::fmt;
use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

/// Ruta relativa a la raíz de la biblioteca.
///
/// Es la única forma de guardar rutas en la base de datos: siempre usa `/`
/// como separador y nunca puede salir de la biblioteca, para que esta siga
/// funcionando al cambiar de equipo, de sistema o de letra de unidad.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelPath(String);

impl RelPath {
    /// Valida una ruta relativa escrita con `/` o `\`.
    pub fn new(path: &str) -> Result<Self> {
        let normalized = path.replace('\\', "/");
        let invalid = || Error::InvalidRelPath(path.to_owned());

        if normalized.is_empty() || normalized.starts_with('/') {
            return Err(invalid());
        }
        let mut parts = Vec::new();
        for part in normalized.split('/') {
            match part {
                "" | "." => continue,
                ".." => return Err(invalid()),
                // Unidades de Windows como "C:".
                p if p.contains(':') => return Err(invalid()),
                p => parts.push(p),
            }
        }
        if parts.is_empty() {
            return Err(invalid());
        }
        Ok(Self(parts.join("/")))
    }

    /// Convierte una ruta absoluta dentro de `root` en una ruta relativa.
    pub fn from_absolute(root: &Path, path: &Path) -> Result<Self> {
        let rest = path
            .strip_prefix(root)
            .map_err(|_| Error::OutsideLibrary(path.to_owned()))?;
        let mut parts = Vec::new();
        for component in rest.components() {
            match component {
                Component::Normal(p) => parts.push(
                    p.to_str()
                        .ok_or_else(|| Error::InvalidRelPath(rest.display().to_string()))?
                        .to_owned(),
                ),
                Component::CurDir => continue,
                _ => return Err(Error::OutsideLibrary(path.to_owned())),
            }
        }
        Self::new(&parts.join("/"))
    }

    /// Ruta absoluta en el equipo actual.
    pub fn to_path(&self, root: &Path) -> PathBuf {
        self.0
            .split('/')
            .fold(root.to_path_buf(), |acc, p| acc.join(p))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normaliza_separadores_y_puntos() {
        let p = RelPath::new(r"Originales\2024\./05\foto.jpg").unwrap();
        assert_eq!(p.as_str(), "Originales/2024/05/foto.jpg");
    }

    #[test]
    fn rechaza_rutas_que_salen_de_la_biblioteca() {
        for bad in [
            "",
            "/etc/passwd",
            "../fuera.jpg",
            "a/../../b",
            r"C:\fotos\a.jpg",
            "C:a.jpg",
            ".",
        ] {
            assert!(RelPath::new(bad).is_err(), "debería rechazar {bad:?}");
        }
    }

    #[test]
    fn ida_y_vuelta_con_rutas_absolutas() {
        let root = Path::new("/media/disco/Biblioteca");
        let abs = root.join("Originales").join("2024").join("foto.jpg");
        let rel = RelPath::from_absolute(root, &abs).unwrap();
        assert_eq!(rel.as_str(), "Originales/2024/foto.jpg");
        assert_eq!(rel.to_path(root), abs);
    }

    #[test]
    fn rechaza_rutas_absolutas_fuera_de_la_raiz() {
        let root = Path::new("/media/disco/Biblioteca");
        assert!(RelPath::from_absolute(root, Path::new("/home/otro/foto.jpg")).is_err());
    }
}
