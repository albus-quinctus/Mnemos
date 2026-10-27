//! Miniaturas de la cuadrícula (spec 002).

use std::io::Cursor;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, GrayImage, ImageFormat, ImageReader, Limits, RgbImage};
use jpeg_decoder::PixelFormat;

/// Alto de una miniatura, en píxeles.
pub const THUMBNAIL_HEIGHT: u32 = 360;
/// Ancho máximo de una miniatura, para que las panorámicas no se disparen.
pub const THUMBNAIL_MAX_WIDTH: u32 = 1080;
/// Calidad JPEG de las miniaturas.
pub const THUMBNAIL_QUALITY: u8 = 80;

/// Memoria máxima que puede reservar el decodificador para una imagen.
const MAX_DECODE_ALLOC: u64 = 512 * 1024 * 1024;

/// Tamaño de salida a partir del tamaño de la imagen ya girada.
type Size = fn(u32, u32) -> (u32, u32);

/// Imagen decodificada, quizá ya reducida, junto con el tamaño completo del
/// original (sin girar), del que sale el tamaño final.
struct Decoded {
    img: DynamicImage,
    full: (u32, u32),
}

impl From<DynamicImage> for Decoded {
    fn from(img: DynamicImage) -> Self {
        let full = (img.width(), img.height());
        Self { img, full }
    }
}

/// Miniatura ya girada y codificada en JPEG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnail {
    pub jpeg: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum ThumbnailError {
    /// Formato que todavía no se decodifica (HEIC, HEIF, AVIF o desconocido)
    /// o RAW sin vista previa incrustada.
    #[error("sin vista previa")]
    Unsupported,
    #[error("no se pudo leer el archivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("no se pudo decodificar la imagen: {0}")]
    Decode(#[from] image::ImageError),
}

/// Genera la miniatura de una imagen JPEG, PNG, WebP, GIF, BMP o TIFF, o de
/// la vista previa incrustada en un RAW.
///
/// `orientation` es la orientación EXIF guardada al indexar (1 a 8); un valor
/// fuera de rango se trata como 1. El formato se reconoce por el contenido, no
/// por la extensión, salvo los RAW: aunque parezcan un TIFF, su miniatura
/// sale siempre de la vista previa que trae el archivo, nunca se revelan.
pub fn thumbnail(path: &Path, orientation: u16) -> Result<Thumbnail, ThumbnailError> {
    render(
        decode(path, orientation, thumbnail_size)?,
        orientation,
        thumbnail_size,
        THUMBNAIL_QUALITY,
    )
}

/// Lado mayor de las imágenes que se generan para el visor.
pub const DISPLAY_MAX_SIDE: u32 = 2560;
/// Calidad JPEG de las imágenes que se generan para el visor.
pub const DISPLAY_QUALITY: u8 = 90;

/// Imagen para el visor cuando el motor web no puede mostrar el original
/// (TIFF, RAW): ya girada, con el lado mayor de [`DISPLAY_MAX_SIDE`] como
/// mucho y en JPEG. Se genera en memoria y no se guarda.
pub fn display_image(path: &Path, orientation: u16) -> Result<Thumbnail, ThumbnailError> {
    render(
        decode(path, orientation, display_size)?,
        orientation,
        display_size,
        DISPLAY_QUALITY,
    )
}

/// Tamaño de la imagen del visor: el lado mayor se limita a
/// [`DISPLAY_MAX_SIDE`] y nunca se amplía.
pub fn display_size(width: u32, height: u32) -> (u32, u32) {
    let (width, height) = (width.max(1), height.max(1));
    let long = width.max(height);
    if long <= DISPLAY_MAX_SIDE {
        return (width, height);
    }
    let scale = |side: u32| {
        ((u64::from(side) * u64::from(DISPLAY_MAX_SIDE) + u64::from(long) / 2) / u64::from(long))
            .max(1) as u32
    };
    (scale(width), scale(height))
}

/// Tipo MIME del archivo si el motor web lo puede mostrar tal cual (JPEG,
/// PNG, WebP, GIF o BMP), según su contenido y no su extensión. Los RAW nunca
/// se muestran tal cual, aunque empiecen como un JPEG.
pub fn web_mime(path: &Path) -> std::io::Result<Option<&'static str>> {
    use std::io::Read;
    if crate::is_raw(path) {
        return Ok(None);
    }
    let mut head = [0u8; 12];
    let n = std::fs::File::open(path)?.read(&mut head)?;
    let head = &head[..n];
    Ok(if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if head.starts_with(b"\x89PNG") {
        Some("image/png")
    } else if head.starts_with(b"GIF8") {
        Some("image/gif")
    } else if head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else if head.starts_with(b"BM") {
        Some("image/bmp")
    } else {
        None
    })
}

/// Decodifica la imagen, o la vista previa incrustada si es un RAW.
///
/// Los JPEG se reducen ya al decodificarlos (ver [`decode_jpeg_reduced`]) al
/// menor tamaño que sigue cubriendo el que pide `size`; si no se puede, se
/// decodifican enteros como el resto de formatos. El archivo se lee una sola
/// vez.
fn decode(path: &Path, orientation: u16, size: Size) -> Result<Decoded, ThumbnailError> {
    let is_raw = crate::is_raw(path);
    let bytes = if is_raw {
        crate::raw_preview(path)?.ok_or(ThumbnailError::Unsupported)?
    } else {
        std::fs::read(path)?
    };
    if !is_raw && !bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
        if reader.format().is_none() {
            return Err(ThumbnailError::Unsupported);
        }
        reader.limits(limits());
        return Ok(reader.decode()?.into());
    }
    if let Some(decoded) = decode_jpeg_reduced(&bytes, orientation, size) {
        return Ok(decoded);
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Jpeg);
    reader.limits(limits());
    Ok(reader.decode()?.into())
}

/// Decodifica un JPEG reducido ya en la IDCT a 1/2, 1/4 o 1/8: el menor de
/// esos tamaños que, tras girarlo, sigue cubriendo el que pide `size`. Evita
/// reconstruir los 24 millones de píxeles de una foto para quedarse con una
/// miniatura de 540 × 360.
///
/// Devuelve `None` cuando no puede: JPEG en CMYK o de 16 bits, o archivo que
/// este decodificador no acepta. Entonces se decodifica entero.
fn decode_jpeg_reduced(jpeg: &[u8], orientation: u16, size: Size) -> Option<Decoded> {
    let mut decoder = jpeg_decoder::Decoder::new(jpeg);
    decoder.set_max_decoding_buffer_size(usize::try_from(MAX_DECODE_ALLOC).ok()?);
    decoder.read_info().ok()?;
    let info = decoder.info()?;
    if !matches!(info.pixel_format, PixelFormat::RGB24 | PixelFormat::L8) {
        return None;
    }
    let full = (u32::from(info.width), u32::from(info.height));
    let (want_w, want_h) = if crate::swaps_dimensions(orientation) {
        let (w, h) = size(full.1, full.0);
        (h, w)
    } else {
        size(full.0, full.1)
    };
    decoder
        .scale(u16::try_from(want_w).ok()?, u16::try_from(want_h).ok()?)
        .ok()?;
    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;
    let (width, height) = (u32::from(info.width), u32::from(info.height));
    let img = match info.pixel_format {
        PixelFormat::RGB24 => DynamicImage::ImageRgb8(RgbImage::from_raw(width, height, pixels)?),
        PixelFormat::L8 => DynamicImage::ImageLuma8(GrayImage::from_raw(width, height, pixels)?),
        _ => return None,
    };
    Some(Decoded { img, full })
}

fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    limits
}

/// Gira, reduce y codifica en JPEG una imagen ya decodificada. El tamaño final
/// sale del original completo, no de la versión reducida, para que no cambie
/// por los redondeos de la reducción.
fn render(
    Decoded { mut img, full }: Decoded,
    orientation: u16,
    size: Size,
    quality: u8,
) -> Result<Thumbnail, ThumbnailError> {
    let swaps = crate::swaps_dimensions(orientation);
    let orientation = u8::try_from(orientation)
        .ok()
        .and_then(Orientation::from_exif)
        .unwrap_or(Orientation::NoTransforms);
    img.apply_orientation(orientation);

    let (width, height) = if swaps {
        size(full.1, full.0)
    } else {
        size(full.0, full.1)
    };
    let small = resize(&img, width, height);
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(Cursor::new(&mut jpeg), quality).encode_image(&small)?;
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

    /// Como [`write_image`], pero en memoria y siempre en JPEG.
    fn write_image_to(out: &mut Vec<u8>, width: u32, height: u32) {
        let img = RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        img.write_to(&mut Cursor::new(out), ImageFormat::Jpeg)
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
    fn la_miniatura_de_un_raw_sale_de_su_vista_previa() {
        use crate::raw::tests::{TiffBuilder, tiny_jpeg};
        let dir = tempdir("raw");
        let path = dir.join("camara.NEF");
        let file = TiffBuilder::new(*b"II")
            .orientation(6)
            .jpeg(tiny_jpeg(8, 8), false)
            .jpeg(tiny_jpeg(800, 600), true)
            .build();
        std::fs::write(&path, file).unwrap();
        let orientation = crate::orientation(&path).unwrap();
        assert_eq!(orientation, 6);
        let thumb = thumbnail(&path, orientation).unwrap();
        assert_eq!(decoded_size(&thumb), (270, 360));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn los_jpeg_se_reducen_al_decodificar() {
        let mut jpeg = Vec::new();
        write_image_to(&mut jpeg, 2400, 1600);
        // 540 × 360 cabe en 1/4 (600 × 400) pero no en 1/8 (300 × 200).
        let img = decode_jpeg_reduced(&jpeg, 1, thumbnail_size).unwrap().img;
        assert_eq!((img.width(), img.height()), (600, 400));
        // En una panorámica el giro decide la reducción: tumbada basta con
        // 1/4 (1200 × 400 para 1080 × 360) y de pie con 1/8 (600 × 200 para
        // 120 × 360 girada, es decir, 360 × 120 sin girar).
        let mut pano = Vec::new();
        write_image_to(&mut pano, 4800, 1600);
        let img = decode_jpeg_reduced(&pano, 1, thumbnail_size).unwrap().img;
        assert_eq!((img.width(), img.height()), (1200, 400));
        let img = decode_jpeg_reduced(&pano, 6, thumbnail_size).unwrap().img;
        assert_eq!((img.width(), img.height()), (600, 200));
        // Una imagen pequeña no se reduce.
        let mut small = Vec::new();
        write_image_to(&mut small, 400, 300);
        let img = decode_jpeg_reduced(&small, 1, thumbnail_size).unwrap().img;
        assert_eq!((img.width(), img.height()), (400, 300));
    }

    #[test]
    fn la_miniatura_reducida_se_parece_a_la_completa() {
        let dir = tempdir("reducida");
        let path = dir.join("foto.jpg");
        write_image(&path, 2400, 1600, ImageFormat::Jpeg);
        let reduced = thumbnail(&path, 1).unwrap();
        let full = render(
            image::open(&path).unwrap().into(),
            1,
            thumbnail_size,
            THUMBNAIL_QUALITY,
        )
        .unwrap();
        assert_eq!((reduced.width, reduced.height), (full.width, full.height));
        let a = image::load_from_memory(&reduced.jpeg).unwrap().to_rgb8();
        let b = image::load_from_memory(&full.jpeg).unwrap().to_rgb8();
        let diff: u64 = a
            .as_raw()
            .iter()
            .zip(b.as_raw())
            .map(|(x, y)| u64::from(x.abs_diff(*y)))
            .sum();
        let mean = diff as f64 / a.as_raw().len() as f64;
        assert!(mean < 3.0, "diferencia media {mean}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn el_tamano_no_depende_de_la_reduccion() {
        // 4001 × 2999 reducido a 1/8 da 501 × 375, cuya miniatura sería de
        // 481 × 360; la del original es de 480 × 360.
        let dir = tempdir("redondeo");
        let path = dir.join("foto.jpg");
        write_image(&path, 4001, 2999, ImageFormat::Jpeg);
        let thumb = thumbnail(&path, 1).unwrap();
        assert_eq!((thumb.width, thumb.height), thumbnail_size(4001, 2999));
        assert_eq!(decoded_size(&thumb), (480, 360));
        let thumb = thumbnail(&path, 6).unwrap();
        assert_eq!((thumb.width, thumb.height), thumbnail_size(2999, 4001));
        assert_eq!(decoded_size(&thumb), (thumb.width, thumb.height));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Recortes y bytes cambiados de un JPEG: el decodificador reducido nunca
    /// debe entrar en pánico, porque la versión final aborta el programa.
    /// `MNEMOS_CASOS_JPEG` sube el número de mutaciones al azar (1000 por
    /// defecto).
    #[test]
    fn los_jpeg_rotos_no_provocan_panico() {
        let mut good = Vec::new();
        write_image_to(&mut good, 48, 32);
        for cut in 0..good.len() {
            let _ = decode_jpeg_reduced(&good[..cut], 1, thumbnail_size);
        }
        for i in 0..good.len() {
            let mut bad = good.clone();
            bad[i] ^= 0xFF;
            let _ = decode_jpeg_reduced(&bad, 6, thumbnail_size);
        }
        let cases: u64 = std::env::var("MNEMOS_CASOS_JPEG")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or(1000);
        let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..cases {
            let mut bad = good.clone();
            for _ in 0..1 + next() % 8 {
                let at = (next() % bad.len() as u64) as usize;
                bad[at] = next() as u8;
            }
            let _ = decode_jpeg_reduced(&bad, 1, thumbnail_size);
        }
    }

    #[test]
    fn los_jpeg_cmyk_se_decodifican_enteros() {
        let jpeg = include_bytes!("../../../tests/fixtures/cmyk.jpg");
        assert!(decode_jpeg_reduced(jpeg, 1, thumbnail_size).is_none());
        let dir = tempdir("cmyk");
        let path = dir.join("imprenta.jpg");
        std::fs::write(&path, jpeg).unwrap();
        assert_eq!(decoded_size(&thumbnail(&path, 1).unwrap()), (16, 16));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn la_imagen_del_visor_se_limita_sin_ampliar() {
        assert_eq!(display_size(6000, 4000), (2560, 1707));
        assert_eq!(display_size(3000, 6000), (1280, 2560));
        assert_eq!(display_size(800, 600), (800, 600));

        let dir = tempdir("visor");
        let path = dir.join("foto.tiff");
        write_image(&path, 3000, 2000, ImageFormat::Tiff);
        let img = display_image(&path, 6).unwrap();
        assert_eq!(decoded_size(&img), (1707, 2560));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reconoce_lo_que_el_motor_web_muestra_tal_cual() {
        let dir = tempdir("mime");
        for (name, format, mime) in [
            ("a.jpg", ImageFormat::Jpeg, Some("image/jpeg")),
            ("b.png", ImageFormat::Png, Some("image/png")),
            ("c.webp", ImageFormat::WebP, Some("image/webp")),
            ("d.gif", ImageFormat::Gif, Some("image/gif")),
            ("e.bmp", ImageFormat::Bmp, Some("image/bmp")),
            ("f.tiff", ImageFormat::Tiff, None),
            // Un JPEG con extensión RAW no se sirve tal cual.
            ("g.nef", ImageFormat::Jpeg, None),
            // Por contenido, no por extensión.
            ("h.png", ImageFormat::Jpeg, Some("image/jpeg")),
        ] {
            let path = dir.join(name);
            write_image(&path, 4, 4, format);
            assert_eq!(web_mime(&path).unwrap(), mime, "{name}");
        }
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
