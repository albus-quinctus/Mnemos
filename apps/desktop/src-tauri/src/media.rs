//! Protocolo `mnemos://` (spec 002): sirve miniaturas y fotos a la interfaz.
//!
//! La interfaz pide las imágenes por el id de la foto, nunca por una ruta:
//! `mnemos://localhost/miniatura/<id>`, `mnemos://localhost/foto/<id>` y
//! `mnemos://localhost/original/<id>`, la foto sin la edición, que es sobre la
//! que trabaja el editor (en
//! Windows, `http://mnemos.localhost/...`). El núcleo traduce el id a su ruta
//! relativa, así que no se puede pedir ningún archivo fuera de la biblioteca.
//!
//! Si la foto está editada (spec 009), las dos imágenes salen con la receta
//! aplicada: la miniatura editada que guarda la cola y la foto generada en
//! memoria. La versión va en la ETag, para que el navegador no reutilice la
//! imagen de antes de editar.
//!
//! Los vídeos (spec 019) se piden con `mnemos://localhost/video/<id>` y se
//! sirven **por trozos** (`Range`, respuesta 206): WebKit y AVFoundation no
//! reproducen ni dejan saltar sin eso, y un vídeo de varios GB no cabe en
//! memoria.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use mnemos_core::PhotoMedia;
use mnemos_core::thumbnails::thumbnail_rel_path_for;
use tauri::http::{Response, StatusCode, header};

/// Qué imagen se pide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Thumbnail,
    Photo,
    /// La foto sin la receta de edición, para el editor (spec 009).
    Original,
    /// El vídeo tal cual, por trozos (spec 019).
    Video,
}

/// Lee `/miniatura/<id>` o `/foto/<id>`. La barra puede llegar codificada
/// (`%2F`), como la genera `convertFileSrc`.
pub fn parse(path: &str) -> Option<(Kind, i64)> {
    let path = path
        .trim_start_matches('/')
        .replace("%2F", "/")
        .replace("%2f", "/");
    let (kind, id) = path.split_once('/')?;
    let kind = match kind {
        "miniatura" => Kind::Thumbnail,
        "foto" => Kind::Photo,
        "original" => Kind::Original,
        "video" => Kind::Video,
        _ => return None,
    };
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((kind, id.parse().ok()?))
}

/// Respuesta para una foto ya localizada en la biblioteca de `root`.
pub fn respond(
    root: &Path,
    kind: Kind,
    media: &PhotoMedia,
    if_none_match: Option<&str>,
) -> Response<Vec<u8>> {
    // La huella identifica el contenido, y la versión la edición: si no
    // cambian, el navegador reutiliza lo que ya tiene.
    let edit = if kind == Kind::Original {
        None
    } else {
        media.edit
    };
    let version = edit.map(|e| e.version);
    let etag = media.content_hash.as_deref().map(|h| {
        let prefix = match kind {
            Kind::Thumbnail => "m",
            Kind::Photo => "f",
            Kind::Original => "o",
            Kind::Video => "v",
        };
        match version {
            Some(v) => format!("\"{prefix}-e{v}-{h}\""),
            None => format!("\"{prefix}-{h}\""),
        }
    });
    if etag.is_some() && etag.as_deref() == if_none_match {
        return build(StatusCode::NOT_MODIFIED, None, etag, Vec::new());
    }
    match kind {
        Kind::Thumbnail => {
            let Some(hash) = media.content_hash.as_deref() else {
                return status(StatusCode::NOT_FOUND);
            };
            match fs::read(root.join(thumbnail_rel_path_for(hash, version))) {
                Ok(jpeg) => build(StatusCode::OK, Some("image/jpeg"), etag, jpeg),
                // Aún no generada: la interfaz muestra el hueco.
                Err(_) => status(StatusCode::NOT_FOUND),
            }
        }
        // Se atiende con `respond_video`, que necesita la cabecera `Range`.
        Kind::Video => respond_video(&media.rel_path.to_path(root), None),
        Kind::Photo | Kind::Original => {
            let path = media.rel_path.to_path(root);
            if let Some(edit) = edit {
                return match mnemos_imaging::display_image_edited(
                    &path,
                    media.orientation,
                    &edit.geometry,
                ) {
                    Ok(img) => build(StatusCode::OK, Some("image/jpeg"), etag, img.jpeg),
                    Err(err) => display_error(&err),
                };
            }
            match mnemos_imaging::web_mime(&path) {
                Ok(Some(mime)) => match fs::read(&path) {
                    Ok(bytes) => build(StatusCode::OK, Some(mime), etag, bytes),
                    Err(_) => status(StatusCode::NOT_FOUND),
                },
                Ok(None) => match mnemos_imaging::display_image(&path, media.orientation) {
                    Ok(img) => build(StatusCode::OK, Some("image/jpeg"), etag, img.jpeg),
                    Err(err) => display_error(&err),
                },
                // El archivo no está en el disco.
                Err(_) => status(StatusCode::NOT_FOUND),
            }
        }
    }
}

/// Mayor trozo de vídeo que se devuelve de una vez. El motor web pide el
/// siguiente cuando lo necesita.
pub const VIDEO_CHUNK: u64 = 4 << 20;

/// Responde a la petición de un vídeo con el trozo que pide `range`
/// (`bytes=inicio-fin`, `bytes=inicio-` o `bytes=-últimos`). Sin `Range`, un
/// vídeo pequeño sale entero y uno grande, su primer trozo.
pub fn respond_video(path: &Path, range: Option<&str>) -> Response<Vec<u8>> {
    let Some(mime) = video_mime(path) else {
        return status(StatusCode::NOT_FOUND);
    };
    let Ok(mut file) = File::open(path) else {
        return status(StatusCode::NOT_FOUND);
    };
    let Ok(total) = file.metadata().map(|m| m.len()) else {
        return status(StatusCode::NOT_FOUND);
    };
    let wanted = match range {
        Some(range) => match parse_range(range, total) {
            Some(r) => Some(r),
            None => {
                return video_response(StatusCode::RANGE_NOT_SATISFIABLE, mime)
                    .header(header::CONTENT_RANGE, format!("bytes */{total}"))
                    .body(Vec::new())
                    .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR));
            }
        },
        None if total <= VIDEO_CHUNK => None,
        None => Some((0, VIDEO_CHUNK - 1)),
    };
    let (start, end) = wanted.unwrap_or((0, total.saturating_sub(1)));
    let end = end.min(start + VIDEO_CHUNK - 1);
    let Ok(len) = usize::try_from(end + 1 - start) else {
        return status(StatusCode::INTERNAL_SERVER_ERROR);
    };
    let mut body = vec![0u8; if total == 0 { 0 } else { len }];
    if file.seek(SeekFrom::Start(start)).is_err() || file.read_exact(&mut body).is_err() {
        // El disco se ha quitado o el archivo ha cambiado a mitad.
        return status(StatusCode::NOT_FOUND);
    }
    let response = if wanted.is_some() {
        video_response(StatusCode::PARTIAL_CONTENT, mime).header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{total}"),
        )
    } else {
        video_response(StatusCode::OK, mime)
    };
    response
        .header(header::CONTENT_LENGTH, body.len())
        .body(body)
        .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR))
}

fn video_response(code: StatusCode, mime: &str) -> tauri::http::response::Builder {
    Response::builder()
        .status(code)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "no-cache")
        .header("X-Content-Type-Options", "nosniff")
        // Para que la interfaz pueda pintar un fotograma en un `<canvas>` y
        // guardarlo como miniatura (spec 019).
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
}

/// Tipo MIME de un vídeo según su extensión, o `None` si no es un vídeo.
fn video_mime(path: &Path) -> Option<&'static str> {
    if !mnemos_imaging::is_video(path) {
        return None;
    }
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "3gp" => "video/3gpp",
        "3g2" => "video/3gpp2",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mts" | "m2ts" => "video/mp2t",
        "wmv" => "video/x-ms-wmv",
        _ => "application/octet-stream",
    })
}

/// Primer intervalo de `bytes=…` dentro de un archivo de `total` bytes, con el
/// final incluido. `None` si no se puede servir (416).
fn parse_range(range: &str, total: u64) -> Option<(u64, u64)> {
    let spec = range.trim().strip_prefix("bytes=")?;
    // Varios intervalos: basta con el primero; el motor web pedirá el resto.
    let first = spec.split(',').next()?.trim();
    let (a, b) = first.split_once('-')?;
    let num = |s: &str| -> Option<u64> {
        (!s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
            .then(|| s.parse().ok())
            .flatten()
    };
    let (start, end) = match (a.trim(), b.trim()) {
        ("", suffix) => {
            let n = num(suffix)?.min(total);
            (total.checked_sub(n)?, total.checked_sub(1)?)
        }
        (start, "") => (num(start)?, total.checked_sub(1)?),
        (start, end) => (num(start)?, num(end)?.min(total.checked_sub(1)?)),
    };
    (start <= end && start < total).then_some((start, end))
}

/// Respuesta cuando no se puede generar la foto del visor.
fn display_error(err: &mnemos_imaging::ThumbnailError) -> Response<Vec<u8>> {
    match err {
        mnemos_imaging::ThumbnailError::Io(_) => status(StatusCode::NOT_FOUND),
        // HEIC sin soporte, RAW sin vista previa o imagen dañada.
        _ => status(StatusCode::UNSUPPORTED_MEDIA_TYPE),
    }
}

pub fn status(code: StatusCode) -> Response<Vec<u8>> {
    build(code, None, None, Vec::new())
}

fn build(
    code: StatusCode,
    content_type: Option<&str>,
    etag: Option<String>,
    body: Vec<u8>,
) -> Response<Vec<u8>> {
    let mut response = Response::builder()
        .status(code)
        .header(header::CACHE_CONTROL, "no-cache")
        .header("X-Content-Type-Options", "nosniff");
    if let Some(content_type) = content_type {
        response = response.header(header::CONTENT_TYPE, content_type);
    }
    if let Some(etag) = etag {
        response = response.header(header::ETAG, etag);
    }
    response.body(body).unwrap_or_else(|_| {
        let mut fallback = Response::new(Vec::new());
        *fallback.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        fallback
    })
}

#[cfg(test)]
mod tests {
    use mnemos_core::Library;

    use super::*;

    #[test]
    fn solo_acepta_ids_numericos() {
        assert_eq!(parse("/miniatura/42"), Some((Kind::Thumbnail, 42)));
        assert_eq!(parse("/foto/7"), Some((Kind::Photo, 7)));
        assert_eq!(parse("/foto%2F7"), Some((Kind::Photo, 7)));
        assert_eq!(parse("/original/7"), Some((Kind::Original, 7)));
        assert_eq!(parse("/video/3"), Some((Kind::Video, 3)));
        for bad in [
            "/",
            "/foto/",
            "/foto/-1",
            "/foto/1/2",
            "/foto/../../etc/passwd",
            "/foto/1.jpg",
            "/Originales/a.jpg",
            "/miniatura/99999999999999999999",
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    fn library_with(name: &str, format: image::ImageFormat) -> (tempfile::TempDir, Library, i64) {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path()).unwrap();
        let path = dir.path().join("Originales").join(name);
        image::RgbImage::from_pixel(3000, 2000, image::Rgb([10, 20, 30]))
            .save_with_format(&path, format)
            .unwrap();
        lib.index().unwrap();
        let id = lib
            .connection()
            .query_row("SELECT id FROM photos", [], |r| r.get(0))
            .unwrap();
        (dir, lib, id)
    }

    #[test]
    fn sirve_el_original_si_el_motor_web_lo_entiende() {
        let (dir, lib, id) = library_with("a.jpg", image::ImageFormat::Jpeg);
        let media = lib.photo_media(id).unwrap().unwrap();
        let response = respond(dir.path(), Kind::Photo, &media, None);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/jpeg");
        let original = fs::read(dir.path().join("Originales/a.jpg")).unwrap();
        assert_eq!(response.body(), &original);

        // Con la misma huella, el navegador reutiliza lo que tiene.
        let etag = response.headers()[header::ETAG]
            .to_str()
            .unwrap()
            .to_owned();
        let again = respond(dir.path(), Kind::Photo, &media, Some(&etag));
        assert_eq!(again.status(), StatusCode::NOT_MODIFIED);
        assert!(again.body().is_empty());
    }

    #[test]
    fn convierte_en_memoria_lo_que_el_motor_web_no_muestra() {
        let (dir, lib, id) = library_with("a.tiff", image::ImageFormat::Tiff);
        let media = lib.photo_media(id).unwrap().unwrap();
        let response = respond(dir.path(), Kind::Photo, &media, None);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/jpeg");
        let img = image::load_from_memory(response.body()).unwrap();
        assert_eq!((img.width(), img.height()), (2560, 1707));
        // No se guarda nada en disco.
        assert_eq!(
            fs::read_dir(dir.path().join(".mnemos/tmp"))
                .map(|d| d.count())
                .unwrap_or(0),
            0
        );
    }

    #[test]
    fn la_miniatura_da_404_hasta_que_existe() {
        let (dir, lib, id) = library_with("a.png", image::ImageFormat::Png);
        let media = lib.photo_media(id).unwrap().unwrap();
        assert_eq!(
            respond(dir.path(), Kind::Thumbnail, &media, None).status(),
            StatusCode::NOT_FOUND
        );
        lib.generate_thumbnails().unwrap();
        let response = respond(dir.path(), Kind::Thumbnail, &media, None);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/jpeg");
    }

    #[test]
    fn una_foto_que_no_esta_en_el_disco_da_404() {
        let (dir, lib, id) = library_with("a.png", image::ImageFormat::Png);
        let media = lib.photo_media(id).unwrap().unwrap();
        fs::remove_file(dir.path().join("Originales/a.png")).unwrap();
        assert_eq!(
            respond(dir.path(), Kind::Photo, &media, None).status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn no_modifica_el_original() {
        let (dir, lib, id) = library_with("a.tiff", image::ImageFormat::Tiff);
        let path = dir.path().join("Originales/a.tiff");
        let before = fs::read(&path).unwrap();
        let mtime = fs::metadata(&path).unwrap().modified().unwrap();
        let media = lib.photo_media(id).unwrap().unwrap();
        respond(dir.path(), Kind::Photo, &media, None);
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), mtime);
    }

    #[test]
    fn una_foto_editada_sale_con_la_receta_y_otra_etag() {
        let (dir, mut lib, id) = library_with("a.jpg", image::ImageFormat::Jpeg);
        let before = respond(
            dir.path(),
            Kind::Photo,
            &lib.photo_media(id).unwrap().unwrap(),
            None,
        );
        let old_etag = before.headers()[header::ETAG].to_str().unwrap().to_owned();
        lib.generate_thumbnails().unwrap();

        let giro = mnemos_imaging::recipe::Geometry {
            rotate: 90,
            ..Default::default()
        };
        lib.save_edit(id, giro).unwrap();
        let media = lib.photo_media(id).unwrap().unwrap();

        // La etiqueta de antes ya no vale: el navegador no reutiliza la foto.
        let response = respond(dir.path(), Kind::Photo, &media, Some(&old_etag));
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/jpeg");
        let etag = response.headers()[header::ETAG]
            .to_str()
            .unwrap()
            .to_owned();
        assert!(etag.starts_with("\"f-e1-"), "{etag}");
        let img = image::load_from_memory(response.body()).unwrap();
        assert_eq!((img.width(), img.height()), (1707, 2560));
        let again = respond(dir.path(), Kind::Photo, &media, Some(&etag));
        assert_eq!(again.status(), StatusCode::NOT_MODIFIED);

        // La miniatura editada da 404 hasta que la genera la cola.
        assert_eq!(
            respond(dir.path(), Kind::Thumbnail, &media, None).status(),
            StatusCode::NOT_FOUND
        );
        lib.generate_thumbnails().unwrap();
        let thumb = respond(dir.path(), Kind::Thumbnail, &media, None);
        assert_eq!(thumb.status(), StatusCode::OK);
        let img = image::load_from_memory(thumb.body()).unwrap();
        assert!(img.height() > img.width());

        // Al volver al original vuelve su miniatura.
        lib.set_edit_version(id, None).unwrap();
        let media = lib.photo_media(id).unwrap().unwrap();
        let thumb = respond(dir.path(), Kind::Thumbnail, &media, None);
        let img = image::load_from_memory(thumb.body()).unwrap();
        assert!(img.width() > img.height());
        let photo = respond(dir.path(), Kind::Photo, &media, Some(&old_etag));
        assert_eq!(photo.status(), StatusCode::NOT_MODIFIED);
    }

    #[test]
    fn el_editor_recibe_la_foto_sin_editar() {
        let (dir, mut lib, id) = library_with("a.jpg", image::ImageFormat::Jpeg);
        let giro = mnemos_imaging::recipe::Geometry {
            rotate: 90,
            ..Default::default()
        };
        lib.save_edit(id, giro).unwrap();
        let media = lib.photo_media(id).unwrap().unwrap();
        let response = respond(dir.path(), Kind::Original, &media, None);
        assert_eq!(response.status(), StatusCode::OK);
        let original = fs::read(dir.path().join("Originales/a.jpg")).unwrap();
        assert_eq!(response.body(), &original);
        let etag = response.headers()[header::ETAG].to_str().unwrap();
        assert!(etag.starts_with("\"o-"), "{etag}");
    }

    fn video_file(bytes: usize) -> (tempfile::TempDir, std::path::PathBuf, Vec<u8>) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("VID_0001.mp4");
        let data: Vec<u8> = (0..bytes).map(|i| (i % 251) as u8).collect();
        fs::write(&path, &data).unwrap();
        (dir, path, data)
    }

    fn header_of(r: &Response<Vec<u8>>, name: header::HeaderName) -> String {
        r.headers()[name].to_str().unwrap().to_owned()
    }

    #[test]
    fn sirve_el_trozo_de_video_que_se_pide() {
        let (_dir, path, data) = video_file(10_000);
        let r = respond_video(&path, Some("bytes=100-199"));
        assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(r.body(), &data[100..200]);
        assert_eq!(header_of(&r, header::CONTENT_RANGE), "bytes 100-199/10000");
        assert_eq!(header_of(&r, header::CONTENT_TYPE), "video/mp4");
        assert_eq!(header_of(&r, header::ACCEPT_RANGES), "bytes");
        assert_eq!(header_of(&r, header::ACCESS_CONTROL_ALLOW_ORIGIN), "*");

        // Desde un punto hasta el final y los últimos bytes.
        let r = respond_video(&path, Some("bytes=9990-"));
        assert_eq!(r.body(), &data[9990..]);
        assert_eq!(
            header_of(&r, header::CONTENT_RANGE),
            "bytes 9990-9999/10000"
        );
        let r = respond_video(&path, Some("bytes=-10"));
        assert_eq!(r.body(), &data[9990..]);
        // Un final de más se recorta al tamaño del archivo.
        let r = respond_video(&path, Some("bytes=0-99999"));
        assert_eq!(r.body(), &data);
        // Sin Range, un vídeo pequeño sale entero.
        let r = respond_video(&path, None);
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(r.body(), &data);
    }

    #[test]
    fn nunca_lee_mas_de_un_trozo() {
        let (_dir, path, _) = video_file(VIDEO_CHUNK as usize + 1000);
        let r = respond_video(&path, Some("bytes=0-"));
        assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(r.body().len() as u64, VIDEO_CHUNK);
        let total = VIDEO_CHUNK + 1000;
        assert_eq!(
            header_of(&r, header::CONTENT_RANGE),
            format!("bytes 0-{}/{total}", VIDEO_CHUNK - 1)
        );
        // Sin Range, uno grande sale por trozos igual.
        let r = respond_video(&path, None);
        assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(r.body().len() as u64, VIDEO_CHUNK);
    }

    #[test]
    fn rechaza_rangos_imposibles() {
        let (_dir, path, _) = video_file(1000);
        for bad in [
            "bytes=1000-",
            "bytes=500-100",
            "bytes=abc",
            "items=0-1",
            "bytes=-0",
        ] {
            let r = respond_video(&path, Some(bad));
            assert_eq!(r.status(), StatusCode::RANGE_NOT_SATISFIABLE, "{bad}");
            assert_eq!(header_of(&r, header::CONTENT_RANGE), "bytes */1000");
        }
    }

    #[test]
    fn solo_sirve_videos_que_estan_en_el_disco() {
        let (dir, path, _) = video_file(10);
        let foto = dir.path().join("a.jpg");
        fs::write(&foto, b"x").unwrap();
        assert_eq!(respond_video(&foto, None).status(), StatusCode::NOT_FOUND);
        fs::remove_file(&path).unwrap();
        assert_eq!(respond_video(&path, None).status(), StatusCode::NOT_FOUND);
    }
}
