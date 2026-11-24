//! Lector XMP mínimo (spec 024): valoración, descripción y palabras clave.
//!
//! Lee el paquete XMP incrustado en JPEG, PNG, WebP y TIFF (también DNG y los
//! RAW con cabecera TIFF), o un archivo `.xmp` suelto. Solo lee: nunca escribe
//! en los archivos ni crea `.xmp`.
//!
//! El XML no se valida contra ningún esquema. No se resuelven DTD ni entidades
//! propias (solo las cinco predefinidas y las de carácter), así que un XMP con
//! entidades no puede crecer ni leer otros archivos. Un XMP mal formado se
//! ignora entero.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use exif::{Context, In, Tag};
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{QName, ResolveResult};
use quick_xml::reader::NsReader;
use quick_xml::{XmlVersion, escape::resolve_predefined_entity};

/// Tamaño máximo de un paquete XMP o de un `.xmp`. Los de las cámaras y de
/// Lightroom ocupan unos pocos KB.
pub const XMP_MAX_BYTES: u64 = 4 * 1024 * 1024;

/// Las palabras clave más largas se descartan.
const KEYWORD_MAX_CHARS: usize = 100;

/// Como mucho se guardan estas palabras clave por foto.
const KEYWORDS_MAX: usize = 200;

const NS_XMP: &str = "http://ns.adobe.com/xap/1.0/";
const NS_DC: &str = "http://purl.org/dc/elements/1.1/";
const NS_RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const NS_XML: &str = "http://www.w3.org/XML/1998/namespace";

/// Cabecera del segmento APP1 de JPEG que lleva el XMP.
const JPEG_XMP_HEADER: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";

/// Etiqueta TIFF con el paquete XMP.
const TIFF_XMP_TAG: u16 = 700;

/// Etiqueta EXIF `Rating` (0 a 5) que escribe Windows.
const EXIF_RATING_TAG: u16 = 0x4746;

/// Lo que MNEMOS usa de un XMP. Cada campo falta si el XMP no lo trae.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XmpData {
    /// `xmp:Rating` de 0 a 5. Un `-1` (rechazada) cuenta como 0.
    pub rating: Option<u8>,
    /// `dc:description`: la entrada `x-default` o, si no la hay, la primera.
    pub description: Option<String>,
    /// `dc:subject`, sin repetidas (sin distinguir mayúsculas), en orden.
    pub keywords: Vec<String>,
}

impl XmpData {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// XMP incrustado en `path`, o `None` si no tiene, no se puede leer o está
/// mal formado. Solo lee el archivo.
pub fn read_xmp(path: &Path) -> Option<XmpData> {
    let packet = xmp_packet(path)?;
    parse_xmp(&packet)
}

/// Contenido de un archivo `.xmp` suelto, o `None` si no existe, es mayor que
/// [`XMP_MAX_BYTES`] o está mal formado. Solo lee el archivo.
pub fn read_xmp_sidecar(path: &Path) -> Option<XmpData> {
    let file = File::open(path).ok()?;
    if file.metadata().ok()?.len() > XMP_MAX_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(XMP_MAX_BYTES).read_to_end(&mut bytes).ok()?;
    parse_xmp(&bytes)
}

/// Valoración de la etiqueta EXIF `Rating` (0 a 5), o `None` si no la hay o
/// no es válida. Solo lee el archivo.
pub fn exif_rating(path: &Path) -> Option<u8> {
    let file = File::open(path).ok()?;
    let exif = exif::Reader::new()
        .read_from_container(&mut BufReader::new(file))
        .ok()?;
    let value = exif
        .get_field(Tag(Context::Tiff, EXIF_RATING_TAG), In::PRIMARY)?
        .value
        .get_uint(0)?;
    u8::try_from(value).ok().filter(|v| *v <= 5)
}

/// Interpreta un paquete XMP. `None` si está mal formado o no trae nada útil.
pub fn parse_xmp(bytes: &[u8]) -> Option<XmpData> {
    if bytes.len() as u64 > XMP_MAX_BYTES {
        return None;
    }
    let text = std::str::from_utf8(strip_bom(bytes)).ok()?;
    let data = parse(text).ok()?;
    (!data.is_empty()).then_some(data)
}

fn strip_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes)
}

/// Propiedad dentro de la que está el lector.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Property {
    Rating,
    Description,
    Subject,
}

/// Fallo al leer el XML: el paquete se ignora entero.
struct Malformed;

impl From<quick_xml::Error> for Malformed {
    fn from(_: quick_xml::Error) -> Self {
        Malformed
    }
}

impl From<quick_xml::events::attributes::AttrError> for Malformed {
    fn from(_: quick_xml::events::attributes::AttrError) -> Self {
        Malformed
    }
}

fn parse(text: &str) -> Result<XmpData, Malformed> {
    let mut reader = NsReader::from_str(text);
    let mut rating: Option<String> = None;
    // (idioma, texto) de cada entrada de la descripción.
    let mut descriptions: Vec<(Option<String>, String)> = Vec::new();
    let mut keywords: Vec<String> = Vec::new();

    // Propiedad abierta y profundidad a la que se abrió.
    let mut property: Option<(Property, usize)> = None;
    // Texto de la propiedad o de la entrada `rdf:li` abierta, y su idioma.
    let mut item: Option<(Option<String>, String)> = None;
    let mut depth = 0usize;

    loop {
        let (ns, event) = reader.read_resolved_event()?;
        let ns = namespace(&ns);
        match event {
            Event::Start(e) => {
                depth += 1;
                let local = e.local_name();
                let local: &str = local.as_ref();
                match (property, ns.as_deref(), local) {
                    (None, Some(NS_RDF), "Description") => {
                        if let Some(value) = rating_attribute(&reader, &e)? {
                            rating = Some(value);
                        }
                    }
                    (None, Some(NS_XMP), "Rating") => {
                        property = Some((Property::Rating, depth));
                        item = Some((None, String::new()));
                    }
                    (None, Some(NS_DC), "description") => {
                        property = Some((Property::Description, depth));
                    }
                    (None, Some(NS_DC), "subject") => {
                        property = Some((Property::Subject, depth));
                    }
                    (Some((Property::Description | Property::Subject, _)), Some(NS_RDF), "li") => {
                        item = Some((lang_attribute(&reader, &e)?, String::new()));
                    }
                    _ => {}
                }
            }
            Event::Empty(e) => {
                let local = e.local_name();
                if ns.as_deref() == Some(NS_RDF)
                    && local.as_ref() == "Description"
                    && property.is_none()
                    && let Some(value) = rating_attribute(&reader, &e)?
                {
                    rating = Some(value);
                }
            }
            Event::Text(e) => {
                if let Some((_, buf)) = item.as_mut() {
                    buf.push_str(&e.xml_content(XmlVersion::Implicit1_0));
                }
            }
            Event::CData(e) => {
                if let Some((_, buf)) = item.as_mut() {
                    buf.push_str(&e);
                }
            }
            Event::GeneralRef(e) => {
                let c = match e.resolve_char_ref()? {
                    Some(c) => c.to_string(),
                    None => {
                        let name: &str = &e;
                        // Solo las entidades predefinidas: nunca las de un DTD.
                        resolve_predefined_entity(name).ok_or(Malformed)?.to_owned()
                    }
                };
                if let Some((_, buf)) = item.as_mut() {
                    buf.push_str(&c);
                }
            }
            Event::End(_) => {
                match property {
                    Some((Property::Rating, d)) if d == depth => {
                        if let Some((_, text)) = item.take() {
                            rating = Some(text);
                        }
                        property = None;
                    }
                    Some((_, d)) if d == depth => {
                        property = None;
                        item = None;
                    }
                    Some((Property::Description, _)) => {
                        if let Some(entry) = item.take() {
                            descriptions.push(entry);
                        }
                    }
                    Some((Property::Subject, _)) => {
                        if let Some((_, text)) = item.take() {
                            keywords.push(text);
                        }
                    }
                    _ => {}
                }
                depth = depth.checked_sub(1).ok_or(Malformed)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if depth != 0 {
        return Err(Malformed);
    }

    Ok(XmpData {
        rating: rating.as_deref().and_then(parse_rating),
        description: pick_description(descriptions),
        keywords: clean_keywords(keywords),
    })
}

fn namespace(ns: &ResolveResult<'_>) -> Option<String> {
    match ns {
        ResolveResult::Bound(n) => Some(n.0.to_owned()),
        _ => None,
    }
}

/// `xmp:Rating` escrito como atributo de `rdf:Description`.
fn rating_attribute(
    reader: &NsReader<&[u8]>,
    e: &BytesStart<'_>,
) -> Result<Option<String>, Malformed> {
    attribute(reader, e, NS_XMP, "Rating")
}

/// `xml:lang` de un `rdf:li`.
fn lang_attribute(
    reader: &NsReader<&[u8]>,
    e: &BytesStart<'_>,
) -> Result<Option<String>, Malformed> {
    for attr in e.attributes() {
        let attr = attr?;
        if attr.key == QName("xml:lang") {
            return Ok(Some(
                attr.normalized_value(XmlVersion::Implicit1_0)?.into_owned(),
            ));
        }
        let (ns, local) = reader.resolver().resolve_attribute(attr.key);
        if namespace(&ns).as_deref() == Some(NS_XML) && local.as_ref() == "lang" {
            return Ok(Some(
                attr.normalized_value(XmlVersion::Implicit1_0)?.into_owned(),
            ));
        }
    }
    Ok(None)
}

fn attribute(
    reader: &NsReader<&[u8]>,
    e: &BytesStart<'_>,
    ns_wanted: &str,
    local_wanted: &str,
) -> Result<Option<String>, Malformed> {
    for attr in e.attributes() {
        let attr = attr?;
        let (ns, local) = reader.resolver().resolve_attribute(attr.key);
        if namespace(&ns).as_deref() == Some(ns_wanted) && local.as_ref() == local_wanted {
            return Ok(Some(
                attr.normalized_value(XmlVersion::Implicit1_0)?.into_owned(),
            ));
        }
    }
    Ok(None)
}

/// "4" → 4, "3.0" → 3, "-1" (rechazada) → 0. Fuera de -1..=5 o sin número: nada.
fn parse_rating(text: &str) -> Option<u8> {
    let value: f64 = text.trim().parse().ok()?;
    if !value.is_finite() || value.fract() != 0.0 {
        return None;
    }
    match value as i64 {
        -1 => Some(0),
        v @ 0..=5 => Some(v as u8),
        _ => None,
    }
}

fn pick_description(entries: Vec<(Option<String>, String)>) -> Option<String> {
    let cleaned: Vec<(Option<String>, String)> = entries
        .into_iter()
        .map(|(lang, text)| (lang, text.trim().to_owned()))
        .filter(|(_, text)| !text.is_empty())
        .collect();
    let default = cleaned
        .iter()
        .position(|(lang, _)| lang.as_deref() == Some("x-default"))
        .unwrap_or(0);
    cleaned.into_iter().nth(default).map(|(_, text)| text)
}

fn clean_keywords(raw: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    raw.into_iter()
        .map(|k| k.trim().to_owned())
        .filter(|k| !k.is_empty() && k.chars().count() <= KEYWORD_MAX_CHARS)
        .filter(|k| seen.insert(k.to_lowercase()))
        .take(KEYWORDS_MAX)
        .collect()
}

/// Paquete XMP incrustado, según la firma del archivo.
fn xmp_packet(path: &Path) -> Option<Vec<u8>> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut magic = [0u8; 12];
    reader.read_exact(&mut magic).ok()?;
    reader.seek(SeekFrom::Start(0)).ok()?;
    match magic {
        [0xFF, 0xD8, ..] => jpeg_packet(&mut reader),
        [0x89, b'P', b'N', b'G', ..] => png_packet(&mut reader),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P'] => webp_packet(&mut reader),
        [b'I', b'I', 42, 0, ..] | [b'M', b'M', 0, 42, ..] => tiff_packet(&mut reader),
        _ => None,
    }
}

fn read_limited(reader: &mut impl Read, len: u64) -> Option<Vec<u8>> {
    if len > XMP_MAX_BYTES {
        return None;
    }
    let mut buf = vec![0; len as usize];
    reader.read_exact(&mut buf).ok()?;
    Some(buf)
}

/// Segmentos APP1 hasta el comienzo de la imagen (SOS).
fn jpeg_packet(reader: &mut (impl BufRead + Seek)) -> Option<Vec<u8>> {
    reader.seek(SeekFrom::Start(2)).ok()?;
    loop {
        let mut marker = [0u8; 2];
        reader.read_exact(&mut marker).ok()?;
        if marker[0] != 0xFF {
            return None;
        }
        match marker[1] {
            // Relleno entre marcadores.
            0xFF => {
                reader.seek(SeekFrom::Current(-1)).ok()?;
                continue;
            }
            // Marcadores sin longitud.
            0x01 | 0xD0..=0xD7 => continue,
            // Empieza la imagen o termina el archivo: no hay XMP.
            0xDA | 0xD9 => return None,
            _ => {}
        }
        let mut len = [0u8; 2];
        reader.read_exact(&mut len).ok()?;
        let len = u16::from_be_bytes(len).checked_sub(2)? as u64;
        if marker[1] == 0xE1 && len > JPEG_XMP_HEADER.len() as u64 {
            let segment = read_limited(reader, len)?;
            if let Some(packet) = segment.strip_prefix(JPEG_XMP_HEADER) {
                return Some(packet.to_vec());
            }
        } else {
            reader.seek(SeekFrom::Current(len as i64)).ok()?;
        }
    }
}

/// Fragmento `iTXt` con la palabra clave `XML:com.adobe.xmp`, sin comprimir.
fn png_packet(reader: &mut (impl BufRead + Seek)) -> Option<Vec<u8>> {
    reader.seek(SeekFrom::Start(8)).ok()?;
    loop {
        let mut head = [0u8; 8];
        reader.read_exact(&mut head).ok()?;
        let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as u64;
        let kind = &head[4..8];
        if kind == b"IEND" {
            return None;
        }
        if kind == b"iTXt" && len <= XMP_MAX_BYTES {
            let data = read_limited(reader, len)?;
            reader.seek(SeekFrom::Current(4)).ok()?; // CRC
            const KEY: &[u8] = b"XML:com.adobe.xmp\0";
            if let Some(rest) = data.strip_prefix(KEY) {
                // Marca de compresión, método, idioma y palabra clave traducida.
                let (&compressed, rest) = rest.split_first()?;
                if compressed != 0 {
                    continue;
                }
                let rest = rest.get(1..)?;
                let lang_end = rest.iter().position(|&b| b == 0)?;
                let rest = &rest[lang_end + 1..];
                let translated_end = rest.iter().position(|&b| b == 0)?;
                return Some(rest[translated_end + 1..].to_vec());
            }
        } else {
            reader.seek(SeekFrom::Current(len as i64 + 4)).ok()?;
        }
    }
}

/// Fragmento `XMP ` de un WebP extendido.
fn webp_packet(reader: &mut (impl BufRead + Seek)) -> Option<Vec<u8>> {
    reader.seek(SeekFrom::Start(12)).ok()?;
    loop {
        let mut head = [0u8; 8];
        reader.read_exact(&mut head).ok()?;
        let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as u64;
        if &head[0..4] == b"XMP " {
            return read_limited(reader, len);
        }
        // Los fragmentos ocupan un número par de bytes.
        reader
            .seek(SeekFrom::Current((len + (len & 1)) as i64))
            .ok()?;
    }
}

/// Etiqueta 700 del primer IFD de un TIFF, DNG o RAW con cabecera TIFF.
fn tiff_packet(reader: &mut (impl BufRead + Seek)) -> Option<Vec<u8>> {
    let exif = exif::Reader::new().read_from_container(reader).ok()?;
    let field = exif.get_field(Tag(Context::Tiff, TIFF_XMP_TAG), In::PRIMARY)?;
    match &field.value {
        exif::Value::Byte(b) | exif::Value::Undefined(b, _) => {
            (b.len() as u64 <= XMP_MAX_BYTES).then(|| b.clone())
        }
        exif::Value::Ascii(v) => v.first().cloned(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const LIGHTROOM: &str = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core 7.0">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmp:Rating="4">
   <dc:description>
    <rdf:Alt>
     <rdf:li xml:lang="es-ES">Otra</rdf:li>
     <rdf:li xml:lang="x-default">Subida al Naranjo &amp; vuelta</rdf:li>
    </rdf:Alt>
   </dc:description>
   <dc:subject>
    <rdf:Bag>
     <rdf:li>Montaña</rdf:li>
     <rdf:li> Picos de Europa </rdf:li>
     <rdf:li>montaña</rdf:li>
     <rdf:li></rdf:li>
    </rdf:Bag>
   </dc:subject>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

    // darktable escribe la valoración como elemento y otros prefijos.
    const DARKTABLE: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:a="http://ns.adobe.com/xap/1.0/" xmlns:b="http://purl.org/dc/elements/1.1/">
   <a:Rating>2</a:Rating>
   <b:subject><rdf:Seq><rdf:li>Gatos</rdf:li></rdf:Seq></b:subject>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#;

    fn xmp(rating: &str) -> String {
        format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="{NS_RDF}"><rdf:Description xmlns:xmp="{NS_XMP}" xmp:Rating="{rating}"/></rdf:RDF></x:xmpmeta>"#
        )
    }

    #[test]
    fn lee_lightroom() {
        let d = parse_xmp(LIGHTROOM.as_bytes()).unwrap();
        assert_eq!(d.rating, Some(4));
        assert_eq!(d.description.as_deref(), Some("Subida al Naranjo & vuelta"));
        assert_eq!(d.keywords, ["Montaña", "Picos de Europa"]);
    }

    #[test]
    fn lee_darktable() {
        let d = parse_xmp(DARKTABLE.as_bytes()).unwrap();
        assert_eq!(d.rating, Some(2));
        assert_eq!(d.description, None);
        assert_eq!(d.keywords, ["Gatos"]);
    }

    #[test]
    fn valoraciones() {
        let r = |s: &str| parse_xmp(xmp(s).as_bytes()).and_then(|d| d.rating);
        assert_eq!(r("5"), Some(5));
        assert_eq!(r("0"), Some(0));
        assert_eq!(r("-1"), Some(0));
        assert_eq!(r("3.0"), Some(3));
        assert_eq!(r("6"), None);
        assert_eq!(r("2.5"), None);
        assert_eq!(r("muchas"), None);
    }

    #[test]
    fn descripcion_sin_x_default_usa_la_primera() {
        let s = format!(
            r#"<rdf:RDF xmlns:rdf="{NS_RDF}"><rdf:Description xmlns:dc="{NS_DC}"><dc:description><rdf:Alt><rdf:li xml:lang="en">Hello</rdf:li><rdf:li xml:lang="es">Hola</rdf:li></rdf:Alt></dc:description></rdf:Description></rdf:RDF>"#
        );
        assert_eq!(
            parse_xmp(s.as_bytes()).unwrap().description.as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn palabras_largas_y_demasiadas() {
        let mut items = format!("<rdf:li>{}</rdf:li>", "x".repeat(101));
        for i in 0..300 {
            items.push_str(&format!("<rdf:li>k{i}</rdf:li>"));
        }
        let s = format!(
            r#"<rdf:RDF xmlns:rdf="{NS_RDF}"><rdf:Description xmlns:dc="{NS_DC}"><dc:subject><rdf:Bag>{items}</rdf:Bag></dc:subject></rdf:Description></rdf:RDF>"#
        );
        let k = parse_xmp(s.as_bytes()).unwrap().keywords;
        assert_eq!(k.len(), KEYWORDS_MAX);
        assert_eq!(k[0], "k0");
    }

    #[test]
    fn ignora_xmp_mal_formado_o_con_entidades() {
        assert_eq!(parse_xmp(b"<x:xmpmeta><rdf:RDF>"), None);
        assert_eq!(parse_xmp(b"no es xml"), None);
        assert_eq!(parse_xmp(&[0xFF, 0xFE, 0x00]), None);
        // Entidad de un DTD: no se resuelve y el paquete se ignora.
        let bomba = format!(
            r#"<!DOCTYPE x [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;">]>
<rdf:RDF xmlns:rdf="{NS_RDF}"><rdf:Description xmlns:dc="{NS_DC}"><dc:subject><rdf:Bag><rdf:li>&b;</rdf:li></rdf:Bag></dc:subject></rdf:Description></rdf:RDF>"#
        );
        assert_eq!(parse_xmp(bomba.as_bytes()), None);
        // Entidad externa: tampoco.
        let externa = format!(
            r#"<!DOCTYPE x [<!ENTITY e SYSTEM "file:///etc/passwd">]><rdf:RDF xmlns:rdf="{NS_RDF}"><rdf:Description xmlns:dc="{NS_DC}"><dc:subject><rdf:Bag><rdf:li>&e;</rdf:li></rdf:Bag></dc:subject></rdf:Description></rdf:RDF>"#
        );
        assert_eq!(parse_xmp(externa.as_bytes()), None);
    }

    #[test]
    fn ignora_xmp_enorme() {
        let mut s = xmp("3").into_bytes();
        s.extend(std::iter::repeat_n(b' ', XMP_MAX_BYTES as usize));
        assert_eq!(parse_xmp(&s), None);
    }

    #[test]
    fn xmp_sin_nada_util_es_none() {
        let s = format!(r#"<rdf:RDF xmlns:rdf="{NS_RDF}"><rdf:Description/></rdf:RDF>"#);
        assert_eq!(parse_xmp(s.as_bytes()), None);
    }

    fn jpeg_with_xmp(packet: &[u8]) -> Vec<u8> {
        let mut out = vec![0xFF, 0xD8];
        // APP0 JFIF cualquiera antes.
        out.extend([0xFF, 0xE0, 0x00, 0x07, b'J', b'F', b'I', b'F', 0]);
        let mut seg = JPEG_XMP_HEADER.to_vec();
        seg.extend_from_slice(packet);
        out.extend([0xFF, 0xE1]);
        out.extend(((seg.len() + 2) as u16).to_be_bytes());
        out.extend(seg);
        out.extend([0xFF, 0xDA, 0x00, 0x02, 0xFF, 0xD9]);
        out
    }

    fn png_with_xmp(packet: &[u8]) -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let chunk = |out: &mut Vec<u8>, kind: &[u8], data: &[u8]| {
            out.extend((data.len() as u32).to_be_bytes());
            out.extend(kind);
            out.extend(data);
            out.extend([0, 0, 0, 0]);
        };
        chunk(&mut out, b"IHDR", &[0; 13]);
        let mut data = b"XML:com.adobe.xmp\0\0\0\0\0".to_vec();
        data.extend_from_slice(packet);
        chunk(&mut out, b"iTXt", &data);
        chunk(&mut out, b"IEND", &[]);
        out
    }

    fn webp_with_xmp(packet: &[u8]) -> Vec<u8> {
        let mut body = b"WEBP".to_vec();
        body.extend(b"VP8X");
        body.extend(10u32.to_le_bytes());
        body.extend([0; 10]);
        body.extend(b"XMP ");
        body.extend((packet.len() as u32).to_le_bytes());
        body.extend_from_slice(packet);
        if packet.len() % 2 == 1 {
            body.push(0);
        }
        let mut out = b"RIFF".to_vec();
        out.extend((body.len() as u32).to_le_bytes());
        out.extend(body);
        out
    }

    /// TIFF mínimo con las etiquetas dadas en el IFD0, en little endian.
    fn tiff_with(entries: &[(u16, u16, Vec<u8>)]) -> Vec<u8> {
        let mut out = b"II\x2a\x00\x08\x00\x00\x00".to_vec();
        let n = entries.len();
        out.extend((n as u16).to_le_bytes());
        let mut data_off = 8 + 2 + n * 12 + 4;
        let mut tail: Vec<u8> = Vec::new();
        for (tag, kind, bytes) in entries {
            out.extend(tag.to_le_bytes());
            out.extend(kind.to_le_bytes());
            let count = if *kind == 3 {
                bytes.len() / 2
            } else {
                bytes.len()
            };
            out.extend((count as u32).to_le_bytes());
            if bytes.len() <= 4 {
                let mut v = bytes.clone();
                v.resize(4, 0);
                out.extend(v);
            } else {
                out.extend((data_off as u32).to_le_bytes());
                tail.extend(bytes);
                data_off += bytes.len();
            }
        }
        out.extend([0, 0, 0, 0]);
        out.extend(tail);
        out
    }

    fn escribe(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = dir.join(name);
        fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn lee_el_xmp_incrustado_de_cada_formato() {
        let dir = tempfile::tempdir().unwrap();
        let packet = LIGHTROOM.as_bytes();
        for (name, bytes) in [
            ("a.jpg", jpeg_with_xmp(packet)),
            ("a.png", png_with_xmp(packet)),
            ("a.webp", webp_with_xmp(packet)),
            (
                "a.dng",
                tiff_with(&[(256, 3, vec![1, 0]), (TIFF_XMP_TAG, 1, packet.to_vec())]),
            ),
        ] {
            let p = escribe(dir.path(), name, &bytes);
            let d = read_xmp(&p).unwrap_or_else(|| panic!("{name} sin XMP"));
            assert_eq!(d.rating, Some(4), "{name}");
            assert_eq!(d.keywords.len(), 2, "{name}");
        }
    }

    #[test]
    fn sin_xmp_o_cortado_es_none() {
        let dir = tempfile::tempdir().unwrap();
        let sin = escribe(
            dir.path(),
            "b.jpg",
            &[0xFF, 0xD8, 0xFF, 0xDA, 0, 2, 0xFF, 0xD9],
        );
        assert_eq!(read_xmp(&sin), None);
        let mut cortado = jpeg_with_xmp(LIGHTROOM.as_bytes());
        cortado.truncate(60);
        let cortado = escribe(dir.path(), "c.jpg", &cortado);
        assert_eq!(read_xmp(&cortado), None);
        let raro = escribe(dir.path(), "d.jpg", b"no es una imagen");
        assert_eq!(read_xmp(&raro), None);
        assert_eq!(read_xmp(&dir.path().join("no-existe.jpg")), None);
    }

    #[test]
    fn lee_un_xmp_suelto() {
        let dir = tempfile::tempdir().unwrap();
        let p = escribe(dir.path(), "IMG_1.CR2.xmp", DARKTABLE.as_bytes());
        assert_eq!(read_xmp_sidecar(&p).unwrap().rating, Some(2));
        assert_eq!(read_xmp_sidecar(&dir.path().join("otro.xmp")), None);
    }

    #[test]
    fn lee_la_valoracion_exif_de_windows() {
        let dir = tempfile::tempdir().unwrap();
        let p = escribe(
            dir.path(),
            "w.tif",
            &tiff_with(&[(256, 3, vec![1, 0]), (EXIF_RATING_TAG, 3, vec![3, 0])]),
        );
        assert_eq!(exif_rating(&p), Some(3));
        let mala = escribe(
            dir.path(),
            "m.tif",
            &tiff_with(&[(256, 3, vec![1, 0]), (EXIF_RATING_TAG, 3, vec![9, 0])]),
        );
        assert_eq!(exif_rating(&mala), None);
        let sin = escribe(dir.path(), "s.tif", &tiff_with(&[(256, 3, vec![1, 0])]));
        assert_eq!(exif_rating(&sin), None);
    }

    #[test]
    fn no_modifica_el_archivo() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = jpeg_with_xmp(LIGHTROOM.as_bytes());
        let p = escribe(dir.path(), "a.jpg", &bytes);
        let antes = fs::metadata(&p).unwrap().modified().unwrap();
        read_xmp(&p).unwrap();
        exif_rating(&p);
        assert_eq!(fs::read(&p).unwrap(), bytes);
        assert_eq!(fs::metadata(&p).unwrap().modified().unwrap(), antes);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
