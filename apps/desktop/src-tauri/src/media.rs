//! Protocolo `mnemos://` (spec 002): sirve miniaturas y fotos a la interfaz.
//!
//! La interfaz pide las imágenes por el id de la foto, nunca por una ruta:
//! `mnemos://localhost/miniatura/<id>` y `mnemos://localhost/foto/<id>` (en
//! Windows, `http://mnemos.localhost/...`). El núcleo traduce el id a su ruta
//! relativa, así que no se puede pedir ningún archivo fuera de la biblioteca.

use std::fs;
use std::path::Path;

use mnemos_core::PhotoMedia;
use mnemos_core::thumbnails::thumbnail_rel_path;
use tauri::http::{Response, StatusCode, header};

/// Qué imagen se pide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Thumbnail,
    Photo,
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
    // La huella identifica el contenido: si no cambia, el navegador reutiliza
    // lo que ya tiene.
    let etag = media.content_hash.as_deref().map(|h| {
        format!(
            "\"{}-{h}\"",
            if kind == Kind::Thumbnail { "m" } else { "f" }
        )
    });
    if etag.is_some() && etag.as_deref() == if_none_match {
        return build(StatusCode::NOT_MODIFIED, None, etag, Vec::new());
    }
    match kind {
        Kind::Thumbnail => {
            let Some(hash) = media.content_hash.as_deref() else {
                return status(StatusCode::NOT_FOUND);
            };
            match fs::read(root.join(thumbnail_rel_path(hash))) {
                Ok(jpeg) => build(StatusCode::OK, Some("image/jpeg"), etag, jpeg),
                // Aún no generada: la interfaz muestra el hueco.
                Err(_) => status(StatusCode::NOT_FOUND),
            }
        }
        Kind::Photo => {
            let path = media.rel_path.to_path(root);
            match mnemos_imaging::web_mime(&path) {
                Ok(Some(mime)) => match fs::read(&path) {
                    Ok(bytes) => build(StatusCode::OK, Some(mime), etag, bytes),
                    Err(_) => status(StatusCode::NOT_FOUND),
                },
                Ok(None) => match mnemos_imaging::display_image(&path, media.orientation) {
                    Ok(img) => build(StatusCode::OK, Some("image/jpeg"), etag, img.jpeg),
                    Err(mnemos_imaging::ThumbnailError::Io(_)) => status(StatusCode::NOT_FOUND),
                    // HEIC sin soporte, RAW sin vista previa o imagen dañada.
                    Err(_) => status(StatusCode::UNSUPPORTED_MEDIA_TYPE),
                },
                // El archivo no está en el disco.
                Err(_) => status(StatusCode::NOT_FOUND),
            }
        }
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
}
