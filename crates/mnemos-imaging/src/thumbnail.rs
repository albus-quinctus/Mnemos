//! Miniaturas de la cuadrícula (spec 002).

use std::io::Cursor;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ImageReader, Limits, RgbImage};

/// Alto de una miniatura, en píxeles.
pub const THUMBNAIL_HEIGHT: u32 = 360;
/// Ancho máximo de una miniatura, para que las panorámicas no se disparen.
pub const THUMBNAIL_MAX_WIDTH: u32 = 1080;
/// Calidad JPEG de las miniaturas.
pub const THUMBNAIL_QUALITY: u8 = 80;

/// Memoria máxima que puede reservar el decodificador para una imagen.
const MAX_DECODE_ALLOC: u64 = 512 * 1024 * 1024;

/// Miniatura ya girada y codificada en JPEG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnail {
    pub jpeg: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum ThumbnailError {
    /// Formato que todavía no se decodifica: RAW, HEIC, HEIF, AVIF o desconocido.
    #[error("formato sin miniatura todavía")]
    Unsupported,
    #[error("no se pudo leer el archivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("no se pudo decodificar la imagen: {0}")]
    Decode(#[from] image::ImageError),
}

/// Genera la miniatura de una imagen JPEG, PNG, WebP, GIF, BMP o TIFF.
///
/// `orientation` es la orientación EXIF guardada al indexar (1 a 8); un valor
/// fuera de rango se trata como 1. El formato se reconoce por el contenido, no
/// por la extensión, salvo los RAW, que se rechazan aunque parezcan un TIFF:
/// su miniatura sale de la vista previa incrustada (tarea 4).
pub fn thumbnail(path: &Path, orientation: u16) -> Result<Thumbnail, ThumbnailError> {
    if crate::is_raw(path) {
        return Err(ThumbnailError::Unsupported);
    }
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    if reader.format().is_none() {
        return Err(ThumbnailError::Unsupported);
    }
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    reader.limits(limits);

    let mut img = reader.decode()?;
    let orientation = u8::try_from(orientation)
        .ok()
        .and_then(Orientation::from_exif)
        .unwrap_or(Orientation::NoTransforms);
    img.apply_orientation(orientation);

    let (width, height) = thumbnail_size(img.width(), img.height());
    let small = resize(&img, width, height);
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(Cursor::new(&mut jpeg), THUMBNAIL_QUALITY)
        .encode_image(&small)?;
    Ok(Thumbnail {
        jpeg,
        width,
        height,
    })
}

/// Tamaño de la miniatura de una imagen ya girada de `width` × `height`.
/// Nunca amplía: una imagen más pequeña conserva su tamaño.
pub fn thumbnail_size(width: u32, height: u32) -> (u32, u32) {
    let (width, height) = (width.max(1), height.max(1));
    let scaled = |num: u32, den: u32, by: u32| {
        ((u64::from(by) * u64::from(num) + u64::from(den) / 2) / u64::from(den)).max(1) as u32
    };
    let (mut w, mut h) = if height > THUMBNAIL_HEIGHT {
        (scaled(width, height, THUMBNAIL_HEIGHT), THUMBNAIL_HEIGHT)
    } else {
        (width, height)
    };
    if w > THUMBNAIL_MAX_WIDTH {
        h = scaled(height, width, THUMBNAIL_MAX_WIDTH);
        w = THUMBNAIL_MAX_WIDTH;
    }
    (w, h)
}

/// Reduce la imagen y la pasa a RGB, con la transparencia sobre fondo blanco.
fn resize(img: &DynamicImage, width: u32, height: u32) -> RgbImage {
    // Primero un muestreo rápido hasta el doble del tamaño final y luego un
    // filtro de calidad: mucho más rápido que filtrar la foto completa.
    let rough = if img.width() > width * 2 && img.height() > height * 2 {
        img.thumbnail_exact(width * 2, height * 2)
    } else {
        img.clone()
    };
    let rgba = rough
        .resize_exact(width, height, FilterType::Triangle)
        .to_rgba8();
    RgbImage::from_fn(width, height, |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let over_white = |c: u8| {
            let (c, a) = (u16::from(c), u16::from(a));
            ((c * a + 255 * (255 - a) + 127) / 255) as u8
        };
        image::Rgb([over_white(r), over_white(g), over_white(b)])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgba, RgbaImage};

    fn tempdir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mnemos-miniaturas-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Escribe una imagen de prueba de `width` × `height` con un degradado.
    fn write_image(path: &Path, width: u32, height: u32, format: ImageFormat) {
        let img = RgbaImage::from_fn(width, height, |x, y| {
            Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255])
        });
        DynamicImage::ImageRgba8(img)
            .to_rgb8()
            .save_with_format(path, format)
            .unwrap();
    }

    fn decoded_size(thumb: &Thumbnail) -> (u32, u32) {
        let img = image::load_from_memory_with_format(&thumb.jpeg, ImageFormat::Jpeg).unwrap();
        (img.width(), img.height())
    }

    #[test]
    fn calcula_el_tamano_sin_ampliar() {
        assert_eq!(thumbnail_size(800, 600), (480, 360));
        assert_eq!(thumbnail_size(600, 800), (270, 360));
        assert_eq!(thumbnail_size(100, 50), (100, 50));
        // Panorámica: el ancho se limita y el alto baja en proporción.
        assert_eq!(thumbnail_size(4000, 400), (1080, 108));
        // Muy estrecha: nunca menos de 1 píxel.
        assert_eq!(thumbnail_size(1, 10_000), (1, 360));
        assert_eq!(thumbnail_size(0, 0), (1, 1));
    }

    #[test]
    fn genera_miniaturas_de_cada_formato() {
        let dir = tempdir("formatos");
        for (name, format) in [
            ("a.jpg", ImageFormat::Jpeg),
            ("b.png", ImageFormat::Png),
            ("c.webp", ImageFormat::WebP),
            ("d.gif", ImageFormat::Gif),
            ("e.bmp", ImageFormat::Bmp),
            ("f.tiff", ImageFormat::Tiff),
        ] {
            let path = dir.join(name);
            write_image(&path, 800, 600, format);
            let thumb = thumbnail(&path, 1).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!((thumb.width, thumb.height), (480, 360), "{name}");
            assert_eq!(decoded_size(&thumb), (480, 360), "{name}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn aplica_la_orientacion() {
        let dir = tempdir("orientacion");
        let path = dir.join("movil.jpg");
        write_image(&path, 800, 600, ImageFormat::Jpeg);
        let thumb = thumbnail(&path, 6).unwrap();
        assert_eq!(decoded_size(&thumb), (270, 360));
        // Un valor fuera de rango no gira nada.
        assert_eq!(decoded_size(&thumbnail(&path, 42).unwrap()), (480, 360));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn la_transparencia_queda_sobre_blanco() {
        let dir = tempdir("transparencia");
        let path = dir.join("logo.png");
        RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 0]))
            .save(&path)
            .unwrap();
        let thumb = thumbnail(&path, 1).unwrap();
        let img = image::load_from_memory(&thumb.jpeg).unwrap().to_rgb8();
        assert!(img.get_pixel(5, 5).0.iter().all(|&c| c > 240));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rechaza_formatos_sin_soporte_y_archivos_rotos() {
        let dir = tempdir("errores");
        // Un RAW basado en TIFF no se decodifica como TIFF.
        let raw = dir.join("camara.dng");
        write_image(&raw, 10, 10, ImageFormat::Tiff);
        assert!(matches!(
            thumbnail(&raw, 1),
            Err(ThumbnailError::Unsupported)
        ));

        let heic = dir.join("iphone.heic");
        std::fs::write(&heic, b"\0\0\0\x18ftypheic").unwrap();
        assert!(matches!(
            thumbnail(&heic, 1),
            Err(ThumbnailError::Unsupported)
        ));

        let truncado = dir.join("truncado.png");
        write_image(&truncado, 50, 50, ImageFormat::Png);
        let bytes = std::fs::read(&truncado).unwrap();
        std::fs::write(&truncado, &bytes[..bytes.len() / 2]).unwrap();
        assert!(matches!(
            thumbnail(&truncado, 1),
            Err(ThumbnailError::Decode(_))
        ));

        assert!(matches!(
            thumbnail(&dir.join("no_existe.jpg"), 1),
            Err(ThumbnailError::Io(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_modifica_el_original() {
        let dir = tempdir("original");
        let path = dir.join("foto.jpg");
        write_image(&path, 800, 600, ImageFormat::Jpeg);
        let before = std::fs::read(&path).unwrap();
        let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
        thumbnail(&path, 6).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), mtime);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
