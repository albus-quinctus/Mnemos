//! Tratamiento de imágenes de MNEMOS.
//!
//! En F0 solo reconoce los formatos de imagen. Las miniaturas, la lectura de
//! EXIF y las recetas de edición llegan en fases posteriores.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconoce_imagenes_sin_importar_mayusculas() {
        assert!(is_supported_image(Path::new("viaje/IMG_0001.JPG")));
        assert!(is_supported_image(Path::new("foto.heic")));
    }

    #[test]
    fn ignora_otros_archivos() {
        assert!(!is_supported_image(Path::new("notas.txt")));
        assert!(!is_supported_image(Path::new("sin_extension")));
        assert!(!is_supported_image(Path::new(".jpg")));
    }
}
