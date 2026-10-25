//! Tratamiento de imágenes de MNEMOS.
//!
//! Reconoce los formatos de imagen y lee sus dimensiones. Las miniaturas, la
//! lectura de EXIF y las recetas de edición llegan en fases posteriores.

use std::path::Path;

/// Extensiones de imagen que MNEMOS reconoce, en minúsculas.
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff", "heic", "heif", "avif",
];

/// Indica si el archivo tiene una extensión de imagen reconocida.
pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|s| s.eq_ignore_ascii_case(ext))
        })
}

/// Ancho y alto en píxeles, leídos solo de la cabecera del archivo.
///
/// Devuelve `None` si la cabecera no se reconoce o está dañada.
pub fn dimensions(path: &Path) -> Option<(u32, u32)> {
    let size = imagesize::size(path).ok()?;
    Some((
        u32::try_from(size.width).ok()?,
        u32::try_from(size.height).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconoce_imagenes_sin_importar_mayusculas() {
        assert!(is_supported_image(Path::new("viaje/IMG_0001.JPG")));
        assert!(is_supported_image(Path::new("foto.heic")));
    }

    #[test]
    fn lee_las_dimensiones_de_la_cabecera() {
        let dir = std::env::temp_dir().join(format!("mnemos-imaging-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("prueba.png");
        std::fs::write(&png, png_header(640, 480)).unwrap();
        assert_eq!(dimensions(&png), Some((640, 480)));

        let roto = dir.join("roto.png");
        std::fs::write(&roto, b"no es un png").unwrap();
        assert_eq!(dimensions(&roto), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Cabecera PNG mínima: firma y bloque IHDR.
    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
        v.extend_from_slice(&13u32.to_be_bytes());
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&width.to_be_bytes());
        v.extend_from_slice(&height.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        v
    }

    #[test]
    fn ignora_otros_archivos() {
        assert!(!is_supported_image(Path::new("notas.txt")));
        assert!(!is_supported_image(Path::new("sin_extension")));
        assert!(!is_supported_image(Path::new(".jpg")));
    }
}
