//! Tratamiento de imágenes de MNEMOS.
//!
//! Reconoce los formatos de imagen, lee sus dimensiones y su orientación, y
//! genera miniaturas. El resto del EXIF y las recetas de edición llegan después.

mod thumbnail;

pub use thumbnail::{
    THUMBNAIL_HEIGHT, THUMBNAIL_MAX_WIDTH, THUMBNAIL_QUALITY, Thumbnail, ThumbnailError, thumbnail,
    thumbnail_size,
};

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Extensiones de imagen que MNEMOS reconoce, en minúsculas.
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff", "heic", "heif", "avif", "dng",
    "cr2", "cr3", "nef", "nrw", "arw", "raf", "orf", "rw2", "pef", "srw",
];

/// Extensiones de los formatos RAW, en minúsculas. Son un subconjunto de
/// [`SUPPORTED_EXTENSIONS`].
pub const RAW_EXTENSIONS: &[&str] = &[
    "dng", "cr2", "cr3", "nef", "nrw", "arw", "raf", "orf", "rw2", "pef", "srw",
];

/// Indica si el archivo tiene una extensión de imagen reconocida.
pub fn is_supported_image(path: &Path) -> bool {
    has_extension(path, SUPPORTED_EXTENSIONS)
}

/// Indica si el archivo es un RAW de cámara, según su extensión.
pub fn is_raw(path: &Path) -> bool {
    has_extension(path, RAW_EXTENSIONS)
}

fn has_extension(path: &Path, list: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| list.iter().any(|s| s.eq_ignore_ascii_case(ext)))
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

/// Orientación EXIF (1 a 8) de la imagen principal.
///
/// Devuelve `None` si el archivo no tiene EXIF, no trae la etiqueta o su
/// valor no es válido; en ese caso la imagen se muestra tal cual (valor 1).
/// Entiende JPEG, TIFF y los RAW basados en TIFF, PNG, WebP y HEIF. Los RAW
/// con contenedor propio (CR3, RAF, ORF y RW2) se cubren con el extractor de
/// vistas previas de la decisión 004.
pub fn orientation(path: &Path) -> Option<u16> {
    let file = File::open(path).ok()?;
    let exif = exif::Reader::new()
        .read_from_container(&mut BufReader::new(file))
        .ok()?;
    let value = exif
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)?
        .value
        .get_uint(0)?;
    u16::try_from(value).ok().filter(|v| (1..=8).contains(v))
}

/// Indica si la orientación gira la imagen 90° o 270°, de modo que su ancho y
/// su alto se intercambian al mostrarla.
pub fn swaps_dimensions(orientation: u16) -> bool {
    (5..=8).contains(&orientation)
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
    fn reconoce_los_raw() {
        for name in [
            "a.DNG", "b.cr3", "c.NEF", "d.arw", "e.RAF", "f.orf", "g.rw2",
        ] {
            assert!(is_supported_image(Path::new(name)), "{name}");
            assert!(is_raw(Path::new(name)), "{name}");
        }
        assert!(!is_raw(Path::new("foto.jpg")));
        assert!(
            RAW_EXTENSIONS
                .iter()
                .all(|e| SUPPORTED_EXTENSIONS.contains(e))
        );
    }

    #[test]
    fn lee_la_orientacion_exif() {
        let dir = tempdir("orientacion");
        let girada = dir.join("girada.jpg");
        std::fs::write(&girada, jpeg_with_orientation(Some(6))).unwrap();
        assert_eq!(orientation(&girada), Some(6));

        let tiff = dir.join("girada.dng");
        std::fs::write(&tiff, tiff_with_orientation(8)).unwrap();
        assert_eq!(orientation(&tiff), Some(8));

        let sin_exif = dir.join("sin_exif.jpg");
        std::fs::write(&sin_exif, jpeg_with_orientation(None)).unwrap();
        assert_eq!(orientation(&sin_exif), None);

        let invalida = dir.join("invalida.jpg");
        std::fs::write(&invalida, jpeg_with_orientation(Some(9))).unwrap();
        assert_eq!(orientation(&invalida), None);

        let rota = dir.join("rota.jpg");
        std::fs::write(&rota, b"no es un jpeg").unwrap();
        assert_eq!(orientation(&rota), None);
        assert_eq!(orientation(&dir.join("no_existe.jpg")), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn las_orientaciones_5_a_8_intercambian_ancho_y_alto() {
        assert!(!(1..=4).any(swaps_dimensions));
        assert!((5..=8).all(swaps_dimensions));
    }

    fn tempdir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mnemos-imaging-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// TIFF mínimo en little endian con una sola entrada: la orientación.
    fn tiff_with_orientation(value: u16) -> Vec<u8> {
        let mut v = b"II*\0".to_vec();
        v.extend_from_slice(&8u32.to_le_bytes()); // desplazamiento del IFD0
        v.extend_from_slice(&1u16.to_le_bytes()); // número de entradas
        v.extend_from_slice(&0x0112u16.to_le_bytes()); // Orientation
        v.extend_from_slice(&3u16.to_le_bytes()); // SHORT
        v.extend_from_slice(&1u32.to_le_bytes()); // un valor
        v.extend_from_slice(&value.to_le_bytes());
        v.extend_from_slice(&[0, 0]); // relleno del valor
        v.extend_from_slice(&0u32.to_le_bytes()); // no hay más IFD
        v
    }

    /// JPEG mínimo con un segmento APP1 EXIF, si se pide una orientación.
    fn jpeg_with_orientation(value: Option<u16>) -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8];
        if let Some(value) = value {
            let mut payload = b"Exif\0\0".to_vec();
            payload.extend_from_slice(&tiff_with_orientation(value));
            v.extend_from_slice(&[0xFF, 0xE1]);
            v.extend_from_slice(&(payload.len() as u16 + 2).to_be_bytes());
            v.extend_from_slice(&payload);
        }
        v.extend_from_slice(&[0xFF, 0xD9]);
        v
    }

    #[test]
    fn ignora_otros_archivos() {
        assert!(!is_supported_image(Path::new("notas.txt")));
        assert!(!is_supported_image(Path::new("sin_extension")));
        assert!(!is_supported_image(Path::new(".jpg")));
    }
}
