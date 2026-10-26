//! Vistas previas JPEG incrustadas en los RAW (decisión 004).
//!
//! No se revela el RAW: se localiza el JPEG que la cámara guardó dentro del
//! archivo y se devuelven sus bytes tal cual. Se leen solo las partes del
//! archivo que hacen falta, con límites en todo lo que viene del archivo para
//! que uno dañado o malicioso nunca provoque un pánico ni agote la memoria.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// Tamaño máximo de una vista previa que se acepta.
const MAX_PREVIEW_LEN: u64 = 64 * 1024 * 1024;
/// Número máximo de IFD que se recorren en un archivo.
const MAX_IFDS: usize = 64;
/// Número máximo de entradas que se leen de un IFD.
const MAX_IFD_ENTRIES: u16 = 1024;
/// Número máximo de cajas ISO-BMFF que se recorren en un CR3.
const MAX_BOXES: usize = 4096;

const TAG_ORIENTATION: u16 = 0x0112;
const TAG_STRIP_OFFSETS: u16 = 0x0111;
const TAG_STRIP_BYTE_COUNTS: u16 = 0x0117;
const TAG_JPEG_OFFSET: u16 = 0x0201;
const TAG_JPEG_LENGTH: u16 = 0x0202;
const TAG_SUB_IFDS: u16 = 0x014A;
const TAG_EXIF_IFD: u16 = 0x8769;
const TAG_MAKER_NOTE: u16 = 0x927C;
/// `JpgFromRaw` de Panasonic (RW2): los bytes del JPEG van en la propia etiqueta.
const TAG_RW2_JPEG: u16 = 0x002E;
/// `CameraSettings` de Olympus, dentro de su MakerNote.
const TAG_OLYMPUS_CAMERA_SETTINGS: u16 = 0x2020;
const TAG_OLYMPUS_PREVIEW_START: u16 = 0x0101;
const TAG_OLYMPUS_PREVIEW_LENGTH: u16 = 0x0102;

/// Caja `uuid` de Canon con la vista previa `PRVW` en un CR3.
const CR3_PREVIEW_UUID: [u8; 16] = [
    0xea, 0xf4, 0x2b, 0x5e, 0x1c, 0x98, 0x4b, 0x88, 0xb9, 0xfb, 0xb7, 0xdc, 0x40, 0x6e, 0x4d, 0x16,
];
/// Caja `uuid` de Canon dentro de `moov` con los metadatos (`CMT1` es el IFD0).
const CR3_METADATA_UUID: [u8; 16] = [
    0x85, 0xc0, 0xb6, 0x87, 0x82, 0x0f, 0x11, 0xe0, 0x81, 0x11, 0xf4, 0xce, 0x46, 0x2b, 0x6a, 0x48,
];

/// Bytes de la vista previa JPEG más grande que trae el RAW, o `None` si no
/// trae ninguna o el formato no se reconoce.
pub fn raw_preview(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let mut src = Source::open(path)?;
    let found = match Container::detect(&mut src)? {
        Container::Tiff => tiff_scan(&mut src, 0)?.preview,
        Container::Raf => raf_preview(&mut src)?,
        Container::Cr3 => cr3_scan(&mut src)?.preview,
        Container::Unknown => None,
    };
    match found {
        Some(span) => Ok(Some(src.read_at(span.offset, span.len)?)),
        None => Ok(None),
    }
}

/// Orientación EXIF que el RAW guarda en su propia estructura. Cubre los
/// contenedores que `kamadak-exif` no entiende: ORF y RW2 (TIFF con otra
/// firma), CR3 (caja `CMT1`) y RAF (EXIF de la vista previa).
pub(crate) fn raw_orientation(path: &Path) -> Option<u16> {
    let mut src = Source::open(path).ok()?;
    match Container::detect(&mut src).ok()? {
        Container::Tiff => tiff_scan(&mut src, 0).ok()?.orientation,
        Container::Cr3 => cr3_scan(&mut src).ok()?.orientation,
        Container::Raf => {
            let span = raf_preview(&mut src).ok()??;
            let jpeg = src.read_at(span.offset, span.len).ok()?;
            crate::orientation_from_container(&mut io::Cursor::new(jpeg))
        }
        Container::Unknown => None,
    }
}

/// Trozo del archivo que contiene un JPEG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Span {
    offset: u64,
    len: u64,
}

#[derive(Debug, Default)]
struct Scan {
    preview: Option<Span>,
    orientation: Option<u16>,
}

impl Scan {
    /// Se queda con la vista previa más grande.
    fn offer(&mut self, src: &mut Source, span: Span) {
        if span.len < 4
            || span.len > MAX_PREVIEW_LEN
            || span.offset.saturating_add(span.len) > src.len
        {
            return;
        }
        if self.preview.is_some_and(|p| p.len >= span.len) {
            return;
        }
        if src
            .read_at(span.offset, 2)
            .is_ok_and(|soi| soi == [0xFF, 0xD8])
        {
            self.preview = Some(span);
        }
    }
}

enum Container {
    Tiff,
    Raf,
    Cr3,
    Unknown,
}

impl Container {
    fn detect(src: &mut Source) -> io::Result<Self> {
        let head = src.read_at(0, 16.min(src.len))?;
        Ok(if head.starts_with(b"FUJIFILMCCD-RAW ") {
            Self::Raf
        } else if head.get(4..8) == Some(b"ftyp") {
            Self::Cr3
        } else if head.starts_with(b"II") || head.starts_with(b"MM") {
            Self::Tiff
        } else {
            Self::Unknown
        })
    }
}

/// Archivo abierto del que se leen trozos con límites.
struct Source {
    file: File,
    len: u64,
}

impl Source {
    fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }

    fn read_at(&mut self, offset: u64, len: u64) -> io::Result<Vec<u8>> {
        let end = offset.checked_add(len).filter(|&end| end <= self.len);
        if end.is_none() || len > MAX_PREVIEW_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "fuera del archivo",
            ));
        }
        let mut buf = vec![0; len as usize];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }
}

/// Orden de bytes de una estructura TIFF.
#[derive(Clone, Copy)]
struct Endian {
    little: bool,
}

impl Endian {
    fn from_mark(mark: &[u8]) -> Option<Self> {
        match mark {
            b"II" => Some(Self { little: true }),
            b"MM" => Some(Self { little: false }),
            _ => None,
        }
    }

    fn u16(self, b: &[u8]) -> u16 {
        let b = [b[0], b[1]];
        if self.little {
            u16::from_le_bytes(b)
        } else {
            u16::from_be_bytes(b)
        }
    }

    fn u32(self, b: &[u8]) -> u32 {
        let b = [b[0], b[1], b[2], b[3]];
        if self.little {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        }
    }
}

/// Entrada de un IFD: etiqueta, tipo, número de valores y el campo de 4 bytes
/// con el valor o su desplazamiento.
#[derive(Clone, Copy)]
struct Entry {
    tag: u16,
    kind: u16,
    count: u32,
    field: [u8; 4],
}

impl Entry {
    /// Primer valor entero de la etiqueta (SHORT, LONG o IFD), si va en línea.
    fn first(&self, e: Endian) -> Option<u32> {
        match self.kind {
            3 => Some(u32::from(e.u16(&self.field))),
            4 | 13 => Some(e.u32(&self.field)),
            _ => None,
        }
    }

    /// Desplazamiento del valor cuando no cabe en línea.
    fn offset(&self, e: Endian) -> u32 {
        e.u32(&self.field)
    }

    /// Todos los valores LONG o IFD (para `SubIFDs`), en línea o no.
    fn longs(&self, src: &mut Source, e: Endian, base: u64) -> Vec<u32> {
        if !matches!(self.kind, 4 | 13) || self.count == 0 || self.count > 64 {
            return Vec::new();
        }
        if self.count == 1 {
            return vec![e.u32(&self.field)];
        }
        let at = base + u64::from(self.offset(e));
        match src.read_at(at, u64::from(self.count) * 4) {
            Ok(bytes) => bytes.as_chunks::<4>().0.iter().map(|c| e.u32(c)).collect(),
            Err(_) => Vec::new(),
        }
    }
}

/// Recorre una estructura TIFF que empieza en `base`: IFD0 y los siguientes,
/// sus SubIFD y la MakerNote de Olympus.
fn tiff_scan(src: &mut Source, base: u64) -> io::Result<Scan> {
    let head = src.read_at(base, 8)?;
    let e = Endian::from_mark(&head[..2])
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no es TIFF"))?;
    let mut scan = Scan::default();
    let mut pending = vec![(u64::from(e.u32(&head[4..8])), true)];
    let mut visited = Vec::new();
    while let Some((ifd, is_ifd0)) = pending.pop() {
        if ifd == 0 || visited.contains(&ifd) || visited.len() >= MAX_IFDS {
            continue;
        }
        visited.push(ifd);
        let Ok((entries, next)) = read_ifd(src, e, base + ifd) else {
            continue;
        };
        pending.push((u64::from(next), false));
        let find = |tag| entries.iter().find(|x| x.tag == tag);

        if is_ifd0 {
            scan.orientation = find(TAG_ORIENTATION)
                .and_then(|x| x.first(e))
                .and_then(|v| u16::try_from(v).ok())
                .filter(|v| (1..=8).contains(v));
        }
        if let (Some(start), Some(len)) = (find(TAG_JPEG_OFFSET), find(TAG_JPEG_LENGTH))
            && let (Some(start), Some(len)) = (start.first(e), len.first(e))
        {
            scan.offer(
                src,
                Span {
                    offset: base + u64::from(start),
                    len: u64::from(len),
                },
            );
        }
        // Una sola tira: en CR2, DNG y otros es un JPEG completo.
        if let (Some(start), Some(len)) = (find(TAG_STRIP_OFFSETS), find(TAG_STRIP_BYTE_COUNTS))
            && start.count == 1
            && len.count == 1
            && let (Some(start), Some(len)) = (start.first(e), len.first(e))
        {
            scan.offer(
                src,
                Span {
                    offset: base + u64::from(start),
                    len: u64::from(len),
                },
            );
        }
        if let Some(jpeg) = find(TAG_RW2_JPEG).filter(|x| x.kind == 7 && x.count > 4) {
            scan.offer(
                src,
                Span {
                    offset: base + u64::from(jpeg.offset(e)),
                    len: u64::from(jpeg.count),
                },
            );
        }
        if let Some(subs) = find(TAG_SUB_IFDS) {
            for sub in subs.longs(src, e, base) {
                pending.push((u64::from(sub), false));
            }
        }
        if let Some(exif) = find(TAG_EXIF_IFD).and_then(|x| x.first(e))
            && let Ok((exif_entries, _)) = read_ifd(src, e, base + u64::from(exif))
            && let Some(note) = exif_entries.iter().find(|x| x.tag == TAG_MAKER_NOTE)
        {
            olympus_preview(src, &mut scan, base + u64::from(note.offset(e)));
        }
    }
    Ok(scan)
}

/// Lee un IFD: sus entradas y el desplazamiento del siguiente.
fn read_ifd(src: &mut Source, e: Endian, at: u64) -> io::Result<(Vec<Entry>, u32)> {
    let count = e.u16(&src.read_at(at, 2)?).min(MAX_IFD_ENTRIES);
    let bytes = src.read_at(at + 2, u64::from(count) * 12 + 4)?;
    let entries = bytes
        .as_chunks::<12>()
        .0
        .iter()
        .map(|c| Entry {
            tag: e.u16(&c[0..2]),
            kind: e.u16(&c[2..4]),
            count: e.u32(&c[4..8]),
            field: [c[8], c[9], c[10], c[11]],
        })
        .collect();
    let next = e.u32(&bytes[bytes.len() - 4..]);
    Ok((entries, next))
}

/// MakerNote de Olympus moderna (`OLYMPUS\0II\x03\0`): la vista previa está en
/// el sub-IFD `CameraSettings`, con desplazamientos relativos a la MakerNote.
fn olympus_preview(src: &mut Source, scan: &mut Scan, note: u64) {
    let Ok(head) = src.read_at(note, 12) else {
        return;
    };
    if &head[..8] != b"OLYMPUS\0" {
        return;
    }
    let Some(e) = Endian::from_mark(&head[8..10]) else {
        return;
    };
    let Ok((entries, _)) = read_ifd(src, e, note + 12) else {
        return;
    };
    let Some(settings) = entries
        .iter()
        .find(|x| x.tag == TAG_OLYMPUS_CAMERA_SETTINGS)
        .map(|x| x.offset(e))
    else {
        return;
    };
    let Ok((entries, _)) = read_ifd(src, e, note + u64::from(settings)) else {
        return;
    };
    let find = |tag| {
        entries
            .iter()
            .find(|x| x.tag == tag)
            .and_then(|x| x.first(e))
    };
    if let (Some(start), Some(len)) = (
        find(TAG_OLYMPUS_PREVIEW_START),
        find(TAG_OLYMPUS_PREVIEW_LENGTH),
    ) {
        scan.offer(
            src,
            Span {
                offset: note + u64::from(start),
                len: u64::from(len),
            },
        );
    }
}

/// RAF de Fujifilm: la cabecera indica dónde está el JPEG.
fn raf_preview(src: &mut Source) -> io::Result<Option<Span>> {
    let head = src.read_at(84, 8)?;
    let offset = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
    let len = u32::from_be_bytes([head[4], head[5], head[6], head[7]]);
    let mut scan = Scan::default();
    scan.offer(
        src,
        Span {
            offset: u64::from(offset),
            len: u64::from(len),
        },
    );
    Ok(scan.preview)
}

/// Caja ISO-BMFF: tipo, inicio de su contenido y fin.
struct Bmff {
    kind: [u8; 4],
    data: u64,
    end: u64,
}

/// Cajas hijas entre `start` y `end`.
fn boxes(src: &mut Source, start: u64, end: u64, budget: &mut usize) -> Vec<Bmff> {
    let mut out = Vec::new();
    let mut at = start;
    while at + 8 <= end && *budget > 0 {
        *budget -= 1;
        let Ok(head) = src.read_at(at, 8) else {
            break;
        };
        let size = u64::from(u32::from_be_bytes([head[0], head[1], head[2], head[3]]));
        let kind = [head[4], head[5], head[6], head[7]];
        let (data, box_end) = match size {
            0 => (at + 8, end),
            1 => match src.read_at(at + 8, 8) {
                Ok(b) => {
                    let large =
                        u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
                    (at + 16, at.saturating_add(large))
                }
                Err(_) => break,
            },
            n => (at + 8, at.saturating_add(n)),
        };
        if box_end <= at || box_end > end || data > box_end {
            break;
        }
        out.push(Bmff {
            kind,
            data,
            end: box_end,
        });
        at = box_end;
    }
    out
}

/// CR3 de Canon: vista previa `PRVW` y orientación de `CMT1`.
fn cr3_scan(src: &mut Source) -> io::Result<Scan> {
    let mut scan = Scan::default();
    let mut budget = MAX_BOXES;
    let len = src.len;
    for top in boxes(src, 0, len, &mut budget) {
        match &top.kind {
            b"uuid" if uuid_is(src, &top, &CR3_PREVIEW_UUID) => {
                // Tras el UUID van 8 bytes y la caja PRVW.
                for prvw in boxes(src, top.data + 24, top.end, &mut budget) {
                    if &prvw.kind == b"PRVW"
                        && let Ok(h) = src.read_at(prvw.data, 16)
                    {
                        let jpeg_len = u32::from_be_bytes([h[12], h[13], h[14], h[15]]);
                        scan.offer(
                            src,
                            Span {
                                offset: prvw.data + 16,
                                len: u64::from(jpeg_len),
                            },
                        );
                    }
                }
            }
            b"moov" => {
                for child in boxes(src, top.data, top.end, &mut budget) {
                    if &child.kind == b"uuid" && uuid_is(src, &child, &CR3_METADATA_UUID) {
                        for meta in boxes(src, child.data + 16, child.end, &mut budget) {
                            if &meta.kind == b"CMT1"
                                && let Ok(tiff) = tiff_scan(src, meta.data)
                            {
                                scan.orientation = tiff.orientation;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(scan)
}

fn uuid_is(src: &mut Source, b: &Bmff, uuid: &[u8; 16]) -> bool {
    b.end >= b.data + 16 && src.read_at(b.data, 16).is_ok_and(|u| u == uuid)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// JPEG mínimo de verdad (1×1) para que se pueda decodificar.
    pub(crate) fn tiny_jpeg(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(width, height, image::Rgb([200, 100, 50]));
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut out)
            .encode_image(&img)
            .unwrap();
        out
    }

    /// Constructor de un TIFF little endian con un IFD0 y, opcionalmente, un
    /// SubIFD. Las vistas previas se añaden al final del archivo.
    pub(crate) struct TiffBuilder {
        magic: [u8; 2],
        ifd0: Vec<(u16, u16, u32, u32)>,
        sub: Vec<(u16, u16, u32, u32)>,
        blobs: Vec<Vec<u8>>,
    }

    impl TiffBuilder {
        pub(crate) fn new(magic: [u8; 2]) -> Self {
            Self {
                magic,
                ifd0: Vec::new(),
                sub: Vec::new(),
                blobs: Vec::new(),
            }
        }

        pub(crate) fn orientation(mut self, value: u16) -> Self {
            self.ifd0.push((TAG_ORIENTATION, 3, 1, u32::from(value)));
            self
        }

        /// Añade un JPEG como `JPEGInterchangeFormat` en el IFD0 o el SubIFD.
        pub(crate) fn jpeg(mut self, jpeg: Vec<u8>, in_sub: bool) -> Self {
            let blob = self.blobs.len() as u32;
            let len = jpeg.len() as u32;
            self.blobs.push(jpeg);
            let ifd = if in_sub {
                &mut self.sub
            } else {
                &mut self.ifd0
            };
            // El desplazamiento se resuelve en `build`: se marca con el índice.
            ifd.push((TAG_JPEG_OFFSET, 4, 1, u32::MAX - blob));
            ifd.push((TAG_JPEG_LENGTH, 4, 1, len));
            self
        }

        /// Añade un JPEG como `JpgFromRaw` de RW2 en el IFD0.
        pub(crate) fn rw2_jpeg(mut self, jpeg: Vec<u8>) -> Self {
            let blob = self.blobs.len() as u32;
            let len = jpeg.len() as u32;
            self.blobs.push(jpeg);
            self.ifd0.push((TAG_RW2_JPEG, 7, len, u32::MAX - blob));
            self
        }

        pub(crate) fn build(mut self) -> Vec<u8> {
            let has_sub = !self.sub.is_empty();
            if has_sub {
                self.ifd0.push((TAG_SUB_IFDS, 4, 1, 0));
            }
            let ifd_len = |n: usize| 2 + n as u32 * 12 + 4;
            let ifd0_at = 8u32;
            let sub_at = ifd0_at + ifd_len(self.ifd0.len());
            let mut blob_at = sub_at + if has_sub { ifd_len(self.sub.len()) } else { 0 };
            let mut offsets = Vec::new();
            for blob in &self.blobs {
                offsets.push(blob_at);
                blob_at += blob.len() as u32;
            }
            let resolve = |v: u32| {
                if v > u32::MAX - 64 {
                    offsets[(u32::MAX - v) as usize]
                } else {
                    v
                }
            };
            let mut out = Vec::new();
            out.extend_from_slice(&self.magic);
            out.extend_from_slice(&[self.magic_tail()[0], self.magic_tail()[1]]);
            out.extend_from_slice(&ifd0_at.to_le_bytes());
            for (ifd, is_ifd0) in [(&self.ifd0, true), (&self.sub, false)] {
                if !is_ifd0 && !has_sub {
                    continue;
                }
                out.extend_from_slice(&(ifd.len() as u16).to_le_bytes());
                for &(tag, kind, count, value) in ifd {
                    let value = if tag == TAG_SUB_IFDS {
                        sub_at
                    } else {
                        resolve(value)
                    };
                    out.extend_from_slice(&tag.to_le_bytes());
                    out.extend_from_slice(&kind.to_le_bytes());
                    out.extend_from_slice(&count.to_le_bytes());
                    if kind == 3 {
                        out.extend_from_slice(&(value as u16).to_le_bytes());
                        out.extend_from_slice(&[0, 0]);
                    } else {
                        out.extend_from_slice(&value.to_le_bytes());
                    }
                }
                out.extend_from_slice(&0u32.to_le_bytes());
            }
            for blob in &self.blobs {
                out.extend_from_slice(blob);
            }
            out
        }

        fn magic_tail(&self) -> [u8; 2] {
            if &self.magic == b"II" {
                [42, 0]
            } else {
                self.magic
            }
        }
    }

    fn write(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mnemos-raw-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn elige_la_vista_previa_mas_grande_de_un_tiff() {
        let small = tiny_jpeg(8, 8);
        let big = tiny_jpeg(64, 48);
        let file = TiffBuilder::new(*b"II")
            .orientation(6)
            .jpeg(small, false)
            .jpeg(big.clone(), true)
            .build();
        let path = write("camara.nef", &file);
        assert_eq!(raw_preview(&path).unwrap(), Some(big));
        assert_eq!(raw_orientation(&path), Some(6));
    }

    #[test]
    fn lee_el_jpeg_de_un_rw2_y_la_orientacion_de_un_orf() {
        let jpeg = tiny_jpeg(16, 16);
        // RW2 usa la firma "IIU\0".
        let rw2 = TiffBuilder::new(*b"II")
            .rw2_jpeg(jpeg.clone())
            .orientation(8)
            .build();
        let mut rw2 = rw2;
        rw2[2] = b'U';
        rw2[3] = 0;
        let path = write("panasonic.rw2", &rw2);
        assert_eq!(raw_preview(&path).unwrap(), Some(jpeg));
        assert_eq!(raw_orientation(&path), Some(8));

        // ORF usa la firma "IIRO".
        let mut orf = TiffBuilder::new(*b"II").orientation(3).build();
        orf[2] = b'R';
        orf[3] = b'O';
        let path = write("olympus.orf", &orf);
        assert_eq!(raw_orientation(&path), Some(3));
        assert_eq!(raw_preview(&path).unwrap(), None);
    }

    #[test]
    fn lee_la_vista_previa_de_un_raf() {
        let jpeg = tiny_jpeg(16, 16);
        let mut raf = b"FUJIFILMCCD-RAW 0201FF383501".to_vec();
        raf.resize(100, 0);
        raf[84..88].copy_from_slice(&100u32.to_be_bytes());
        raf[88..92].copy_from_slice(&(jpeg.len() as u32).to_be_bytes());
        raf.extend_from_slice(&jpeg);
        let path = write("fuji.raf", &raf);
        assert_eq!(raw_preview(&path).unwrap(), Some(jpeg));
    }

    fn bmff(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn lee_la_vista_previa_y_la_orientacion_de_un_cr3() {
        let jpeg = tiny_jpeg(32, 24);
        let mut prvw = vec![0u8; 12];
        prvw.extend_from_slice(&(jpeg.len() as u32).to_be_bytes());
        prvw.extend_from_slice(&jpeg);
        let mut preview_uuid = CR3_PREVIEW_UUID.to_vec();
        preview_uuid.extend_from_slice(&[0; 8]);
        preview_uuid.extend_from_slice(&bmff(b"PRVW", &prvw));

        let cmt1 = TiffBuilder::new(*b"II").orientation(6).build();
        let mut meta_uuid = CR3_METADATA_UUID.to_vec();
        meta_uuid.extend_from_slice(&bmff(b"CMT1", &cmt1));

        let mut file = bmff(b"ftyp", b"crx \0\0\0\x01");
        file.extend_from_slice(&bmff(b"moov", &bmff(b"uuid", &meta_uuid)));
        file.extend_from_slice(&bmff(b"uuid", &preview_uuid));
        let path = write("canon.cr3", &file);
        assert_eq!(raw_preview(&path).unwrap(), Some(jpeg));
        assert_eq!(raw_orientation(&path), Some(6));
    }

    #[test]
    fn los_archivos_rotos_no_provocan_panico() {
        let jpeg = tiny_jpeg(16, 16);
        let good = TiffBuilder::new(*b"II")
            .orientation(6)
            .jpeg(jpeg, true)
            .build();
        // Todos los recortes posibles del archivo, y bytes cambiados uno a uno.
        for cut in 0..good.len() {
            let path = write("recorte.dng", &good[..cut]);
            let _ = raw_preview(&path);
            let _ = raw_orientation(&path);
        }
        for i in 0..good.len().min(200) {
            let mut bad = good.clone();
            bad[i] ^= 0xFF;
            let path = write("cambiado.dng", &bad);
            let _ = raw_preview(&path);
            let _ = raw_orientation(&path);
        }
        // Un IFD que apunta a sí mismo no hace un bucle infinito.
        let mut ciclo = b"II*\0\x08\0\0\0\x00\0\x08\0\0\0".to_vec();
        ciclo.extend_from_slice(&[0; 8]);
        let path = write("ciclo.nef", &ciclo);
        assert_eq!(raw_preview(&path).unwrap(), None);
        // Un JPEG que dice ser enorme se ignora.
        let mut raf = b"FUJIFILMCCD-RAW ".to_vec();
        raf.resize(100, 0);
        raf[84..88].copy_from_slice(&100u32.to_be_bytes());
        raf[88..92].copy_from_slice(&u32::MAX.to_be_bytes());
        let path = write("enorme.raf", &raf);
        assert_eq!(raw_preview(&path).unwrap(), None);
    }

    #[test]
    fn un_formato_desconocido_no_tiene_vista_previa() {
        let path = write("texto.dng", b"esto no es un raw");
        assert_eq!(raw_preview(&path).unwrap(), None);
        assert_eq!(raw_orientation(&path), None);
    }
}
