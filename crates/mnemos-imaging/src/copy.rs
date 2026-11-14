//! Foto editada a tamaño completo para «Guardar como copia» (spec 009).

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

use exif::{Field, In, Tag, Value};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{DynamicImage, ExtendedColorType, ImageEncoder};

use crate::recipe::{Geometry, Transform, crop_rect};
use crate::thumbnail::{Decoded, ThumbnailError, decode, flatten};

/// Calidad JPEG de las copias editadas.
pub const COPY_QUALITY: u8 = 95;

/// Formato de una copia editada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyFormat {
    Jpeg,
    Png,
}

impl CopyFormat {
    /// Extensión que le corresponde, en minúsculas y sin punto.
    pub fn extension(self) -> &'static str {
        match self {
            CopyFormat::Jpeg => "jpg",
            CopyFormat::Png => "png",
        }
    }

    /// Indica si `ext` (sin punto, en cualquier caja) es una extensión de
    /// este formato.
    pub fn matches_extension(self, ext: &str) -> bool {
        let ext = ext.to_ascii_lowercase();
        match self {
            CopyFormat::Jpeg => {
                matches!(
                    ext.as_str(),
                    "jpg" | "jpeg" | "jpe" | "jfif" | "jfi" | "jif"
                )
            }
            CopyFormat::Png => ext == "png",
        }
    }
}

/// Copia editada ya codificada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditedCopy {
    pub bytes: Vec<u8>,
    pub format: CopyFormat,
    pub width: u32,
    pub height: u32,
}

/// Genera la foto editada a tamaño completo: orientada según el EXIF, con la
/// geometría aplicada y sin reducir.
///
/// Sale en PNG si el original es PNG, para no perder la transparencia, y en
/// JPEG de calidad [`COPY_QUALITY`] en el resto de casos (JPEG, RAW, TIFF,
/// WebP, GIF, BMP). Lleva el EXIF del original con la orientación a 1 y las
/// dimensiones nuevas; si no se puede leer o escribir, sale sin EXIF.
pub fn edited_copy(
    path: &Path,
    orientation: u16,
    geometry: &Geometry,
) -> Result<EditedCopy, ThumbnailError> {
    let format = if !crate::is_raw(path) && is_png(path)? {
        CopyFormat::Png
    } else {
        CopyFormat::Jpeg
    };
    let Decoded { mut img, .. } = decode(path, orientation, &|w, h| (w, h))?;
    let transform = Transform::of(orientation, geometry);
    if let Some(crop) = &geometry.crop {
        let (x, y, w, h) = crop_rect(img.width(), img.height(), &transform.source_crop(crop));
        img = img.crop_imm(x, y, w, h);
    }
    let img = match format {
        CopyFormat::Png if img.color().has_alpha() => {
            DynamicImage::ImageRgba8(transform.apply(img.into_rgba8()))
        }
        _ => DynamicImage::ImageRgb8(transform.apply(flatten(img))),
    };
    let (width, height) = (img.width(), img.height());
    let exif = copied_exif(path, width, height);

    let mut bytes = Vec::new();
    let color = ExtendedColorType::from(img.color());
    match format {
        CopyFormat::Jpeg => {
            let mut encoder = JpegEncoder::new_with_quality(Cursor::new(&mut bytes), COPY_QUALITY);
            if let Some(exif) = exif {
                // El codificador JPEG siempre admite EXIF.
                let _ = encoder.set_exif_metadata(exif);
            }
            encoder.write_image(img.as_bytes(), width, height, color)?;
        }
        CopyFormat::Png => {
            let mut encoder = PngEncoder::new(Cursor::new(&mut bytes));
            if let Some(exif) = exif {
                let _ = encoder.set_exif_metadata(exif);
            }
            encoder.write_image(img.as_bytes(), width, height, color)?;
        }
    }
    Ok(EditedCopy {
        bytes,
        format,
        width,
        height,
    })
}

fn is_png(path: &Path) -> std::io::Result<bool> {
    use std::io::Read;
    let mut head = [0u8; 8];
    let n = File::open(path)?.read(&mut head)?;
    Ok(head[..n].starts_with(b"\x89PNG"))
}

/// Campos de TIFF que describen cómo están guardados los píxeles del
/// original. No valen para la copia, que tiene otros.
const IMAGE_STRUCTURE: &[Tag] = &[
    Tag::ImageWidth,
    Tag::ImageLength,
    Tag::BitsPerSample,
    Tag::Compression,
    Tag::PhotometricInterpretation,
    Tag::SamplesPerPixel,
    Tag::RowsPerStrip,
    Tag::PlanarConfiguration,
    Tag::YCbCrSubSampling,
    Tag::Orientation,
    // Datos del fabricante con desplazamientos que no sobreviven a una copia.
    Tag::MakerNote,
    Tag::PixelXDimension,
    Tag::PixelYDimension,
];

/// EXIF del original listo para la copia (TIFF sin la cabecera `Exif\0\0`):
/// sin miniatura, sin los campos de estructura de la imagen ni los de
/// fabricante, con la orientación a 1 y las dimensiones de la copia. `None`
/// si el original no tiene EXIF o no se puede escribir.
pub(crate) fn copied_exif(path: &Path, width: u32, height: u32) -> Option<Vec<u8>> {
    let file = File::open(path).ok()?;
    let exif = exif::Reader::new()
        .read_from_container(&mut BufReader::new(file))
        .ok()?;
    let mut fields: Vec<Field> = exif
        .fields()
        .filter(|f| {
            f.ifd_num == In::PRIMARY
                && f.tag.description().is_some()
                && !IMAGE_STRUCTURE.contains(&f.tag)
                && !matches!(f.value, Value::Unknown(..))
        })
        .cloned()
        .collect();
    fields.push(Field {
        tag: Tag::Orientation,
        ifd_num: In::PRIMARY,
        value: Value::Short(vec![1]),
    });
    fields.push(Field {
        tag: Tag::PixelXDimension,
        ifd_num: In::PRIMARY,
        value: Value::Long(vec![width]),
    });
    fields.push(Field {
        tag: Tag::PixelYDimension,
        ifd_num: In::PRIMARY,
        value: Value::Long(vec![height]),
    });
    let mut writer = exif::experimental::Writer::new();
    for field in &fields {
        writer.push_field(field);
    }
    let mut out = Cursor::new(Vec::new());
    writer.write(&mut out, exif.little_endian()).ok()?;
    Some(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::Crop;
    use image::{ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};

    fn tempdir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mnemos-copia-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// TIFF mínimo con orientación 6, fecha de captura y una miniatura (IFD1).
    fn exif_tiff() -> Vec<u8> {
        let fields = [
            Field {
                tag: Tag::Orientation,
                ifd_num: In::PRIMARY,
                value: Value::Short(vec![6]),
            },
            Field {
                tag: Tag::Make,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"Camara".to_vec()]),
            },
            Field {
                tag: Tag::DateTimeOriginal,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"2021:07:14 18:30:05".to_vec()]),
            },
            Field {
                tag: Tag::MakerNote,
                ifd_num: In::PRIMARY,
                value: Value::Undefined(vec![1, 2, 3, 4], 0),
            },
            Field {
                tag: Tag::PixelXDimension,
                ifd_num: In::PRIMARY,
                value: Value::Long(vec![6]),
            },
            Field {
                tag: Tag::Compression,
                ifd_num: In::THUMBNAIL,
                value: Value::Short(vec![6]),
            },
        ];
        let thumb = [0xFF, 0xD8, 0xFF, 0xD9];
        let mut writer = exif::experimental::Writer::new();
        for field in &fields {
            writer.push_field(field);
        }
        writer.set_jpeg(&thumb, In::THUMBNAIL);
        let mut out = Cursor::new(Vec::new());
        writer.write(&mut out, false).unwrap();
        out.into_inner()
    }

    /// JPEG de 60 × 40 con el EXIF de [`exif_tiff`]: rojo a la izquierda y azul
    /// a la derecha.
    fn write_jpeg_with_exif(path: &Path) {
        let img = RgbImage::from_fn(60, 40, |x, _| {
            if x < 30 {
                Rgb([250, 0, 0])
            } else {
                Rgb([0, 0, 250])
            }
        });
        let mut bytes = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(Cursor::new(&mut bytes), 95);
        encoder.set_exif_metadata(exif_tiff()).unwrap();
        encoder
            .write_image(img.as_raw(), 60, 40, ExtendedColorType::Rgb8)
            .unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn read_exif(bytes: &[u8]) -> exif::Exif {
        exif::Reader::new()
            .read_from_container(&mut Cursor::new(bytes))
            .unwrap()
    }

    fn uint(exif: &exif::Exif, tag: Tag) -> Option<u32> {
        exif.get_field(tag, In::PRIMARY)?.value.get_uint(0)
    }

    #[test]
    fn la_copia_de_un_jpeg_va_a_tamano_completo_con_el_exif_y_la_orientacion_a_1() {
        let dir = tempdir("jpeg");
        let path = dir.join("foto.jpg");
        write_jpeg_with_exif(&path);
        let geometry = Geometry {
            rotate: 90,
            ..Geometry::default()
        };
        // EXIF 6 (90° horario) y otros 90°: 180° en total, 60 × 40.
        let copy = edited_copy(&path, 6, &geometry).unwrap();
        assert_eq!(copy.format, CopyFormat::Jpeg);
        assert_eq!((copy.width, copy.height), (60, 40));
        let img = image::load_from_memory(&copy.bytes).unwrap().into_rgb8();
        assert_eq!(img.dimensions(), (60, 40));
        // Girada 180°: el azul queda a la izquierda.
        assert!(img.get_pixel(5, 20)[2] > 200, "{:?}", img.get_pixel(5, 20));
        assert!(
            img.get_pixel(55, 20)[0] > 200,
            "{:?}",
            img.get_pixel(55, 20)
        );

        let exif = read_exif(&copy.bytes);
        assert_eq!(uint(&exif, Tag::Orientation), Some(1));
        assert_eq!(uint(&exif, Tag::PixelXDimension), Some(60));
        assert_eq!(uint(&exif, Tag::PixelYDimension), Some(40));
        let date = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY).unwrap();
        assert_eq!(date.display_value().to_string(), "2021-07-14 18:30:05");
        assert!(exif.get_field(Tag::Make, In::PRIMARY).is_some());
        assert!(exif.get_field(Tag::MakerNote, In::PRIMARY).is_none());
        assert!(exif.fields().all(|f| f.ifd_num == In::PRIMARY));
        assert_eq!(
            crate::orientation_from_container(&mut Cursor::new(&copy.bytes)),
            Some(1)
        );
    }

    #[test]
    fn la_copia_de_un_png_sigue_en_png_con_su_transparencia() {
        let dir = tempdir("png");
        let path = dir.join("foto.png");
        RgbaImage::from_fn(10, 20, |_, y| {
            if y < 10 {
                Rgba([0, 200, 0, 0])
            } else {
                Rgba([0, 200, 0, 255])
            }
        })
        .save_with_format(&path, ImageFormat::Png)
        .unwrap();
        let geometry = Geometry {
            crop: Some(Crop {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 0.75,
            }),
            ..Geometry::default()
        };
        let copy = edited_copy(&path, 1, &geometry).unwrap();
        assert_eq!(copy.format, CopyFormat::Png);
        assert_eq!((copy.width, copy.height), (10, 15));
        let img = image::load_from_memory_with_format(&copy.bytes, ImageFormat::Png)
            .unwrap()
            .into_rgba8();
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(0, 14)[3], 255);
    }

    #[test]
    fn sin_exif_en_el_original_la_copia_sale_sin_exif() {
        let dir = tempdir("bmp");
        let path = dir.join("foto.bmp");
        RgbImage::new(8, 4)
            .save_with_format(&path, ImageFormat::Bmp)
            .unwrap();
        let copy = edited_copy(&path, 1, &Geometry::default()).unwrap();
        assert_eq!(copy.format, CopyFormat::Jpeg);
        assert_eq!((copy.width, copy.height), (8, 4));
        assert!(
            exif::Reader::new()
                .read_from_container(&mut Cursor::new(&copy.bytes))
                .is_err()
        );
    }

    #[test]
    fn las_extensiones_de_cada_formato() {
        assert!(CopyFormat::Jpeg.matches_extension("JPG"));
        assert!(CopyFormat::Jpeg.matches_extension("jpeg"));
        assert!(CopyFormat::Jpeg.matches_extension("JFIF"));
        assert!(!CopyFormat::Jpeg.matches_extension("cr2"));
        assert!(CopyFormat::Png.matches_extension("PNG"));
        assert_eq!(CopyFormat::Jpeg.extension(), "jpg");
    }
}
