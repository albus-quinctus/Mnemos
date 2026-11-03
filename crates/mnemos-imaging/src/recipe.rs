//! Recetas de edición no destructiva (spec 009).
//!
//! Una receta describe cómo se ve una foto editada sin tocar el original: se
//! guarda como JSON en `.mnemos/ediciones/` y se aplica cada vez que se genera
//! una miniatura o la imagen del visor. Este módulo define el formato 1 y
//! aplica su geometría (giro, volteo y recorte) a una imagen ya girada según
//! su orientación EXIF.

use image::DynamicImage;
use serde::{Deserialize, Serialize};

/// Versión del formato de receta que entiende y escribe esta versión de
/// MNEMOS.
pub const RECIPE_FORMAT: u32 = 1;

/// Margen para los redondeos de las fracciones del recorte al validarlas.
const CROP_EPSILON: f64 = 1e-6;

/// Receta de una foto: todas sus versiones y cuál se muestra.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    /// Versión del formato. Una receta de un formato mayor que
    /// [`RECIPE_FORMAT`] se puede leer, pero no se debe guardar encima.
    pub format: u32,
    /// Original al que pertenece la receta.
    pub original: OriginalRef,
    /// Id de la versión que se muestra, o `None` para mostrar el original.
    pub current: Option<u32>,
    /// Versiones guardadas, de la más antigua a la más nueva.
    pub versions: Vec<Version>,
}

/// Datos del original para comprobar que la receta es suya.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginalRef {
    /// Huella BLAKE3 del contenido, en hexadecimal.
    pub hash: String,
    /// Ancho del original ya girado según el EXIF.
    pub width: u32,
    /// Alto del original ya girado según el EXIF.
    pub height: u32,
}

/// Estado completo del editor al guardar. Cambiar de versión es solo cambiar
/// [`Recipe::current`]: una versión no depende de la anterior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub id: u32,
    /// Cuándo se guardó, en segundos Unix (UTC).
    pub created_at: i64,
    #[serde(default)]
    pub geometry: Geometry,
}

/// Giro, volteo y recorte. Se aplican en ese orden sobre la imagen ya girada
/// según el EXIF.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Geometry {
    /// Giro en sentido horario: 0, 90, 180 o 270 grados.
    #[serde(default)]
    pub rotate: u16,
    /// Reflejo horizontal, después del giro.
    #[serde(default)]
    pub flip_horizontal: bool,
    /// Recorte sobre la imagen ya girada y volteada, o `None` si no hay.
    #[serde(default)]
    pub crop: Option<Crop>,
}

/// Rectángulo de recorte en fracciones (0 a 1) del ancho y el alto. Al no
/// depender de los píxeles, sirve igual para el original que para una
/// versión reducida.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Crop {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Por qué no se puede usar una receta.
#[derive(Debug, thiserror::Error)]
pub enum RecipeError {
    #[error("la receta no es un JSON válido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("receta no válida: {0}")]
    Invalid(String),
}

fn invalid(reason: impl Into<String>) -> RecipeError {
    RecipeError::Invalid(reason.into())
}

impl Recipe {
    /// Receta nueva, sin versiones, que muestra el original.
    pub fn new(original: OriginalRef) -> Self {
        Self {
            format: RECIPE_FORMAT,
            original,
            current: None,
            versions: Vec::new(),
        }
    }

    /// Lee y valida una receta. Los campos que no conoce se ignoran, para que
    /// las recetas de versiones futuras se puedan mostrar.
    pub fn parse(json: &[u8]) -> Result<Self, RecipeError> {
        let recipe: Self = serde_json::from_slice(json)?;
        recipe.validate()?;
        Ok(recipe)
    }

    /// Receta en JSON legible, lista para guardar.
    pub fn to_json(&self) -> Result<Vec<u8>, RecipeError> {
        self.validate()?;
        let mut json = serde_json::to_vec_pretty(self)?;
        json.push(b'\n');
        Ok(json)
    }

    /// Indica si esta versión de MNEMOS puede guardar sobre la receta sin
    /// perder nada que no entiende.
    pub fn is_writable(&self) -> bool {
        self.format == RECIPE_FORMAT
    }

    /// Comprueba los valores: formato, versiones únicas, `current` existente,
    /// giros y recortes dentro de rango.
    pub fn validate(&self) -> Result<(), RecipeError> {
        if self.format == 0 {
            return Err(invalid("formato 0"));
        }
        if self.original.width == 0 || self.original.height == 0 {
            return Err(invalid("el original no tiene tamaño"));
        }
        if self.original.hash.is_empty()
            || !self.original.hash.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(invalid("huella del original no válida"));
        }
        let mut ids: Vec<u32> = self.versions.iter().map(|v| v.id).collect();
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(invalid("hay dos versiones con el mismo id"));
        }
        if let Some(current) = self.current
            && !ids.contains(&current)
        {
            return Err(invalid(format!("no existe la versión {current}")));
        }
        for version in &self.versions {
            version
                .geometry
                .validate()
                .map_err(|e| invalid(format!("versión {}: {e}", version.id)))?;
        }
        Ok(())
    }

    /// Versión que se muestra, o `None` si se muestra el original.
    pub fn current_version(&self) -> Option<&Version> {
        let id = self.current?;
        self.versions.iter().find(|v| v.id == id)
    }

    /// Geometría que se muestra; la identidad si se muestra el original.
    pub fn current_geometry(&self) -> Geometry {
        self.current_version()
            .map(|v| v.geometry)
            .unwrap_or_default()
    }

    /// Añade una versión con esta geometría, la hace actual y devuelve su id.
    pub fn add_version(&mut self, geometry: Geometry, created_at: i64) -> Result<u32, RecipeError> {
        if !self.is_writable() {
            return Err(invalid(format!(
                "la receta es del formato {}, más nuevo que el {RECIPE_FORMAT}",
                self.format
            )));
        }
        geometry.validate().map_err(invalid)?;
        let id = self.versions.iter().map(|v| v.id).max().unwrap_or(0) + 1;
        self.versions.push(Version {
            id,
            created_at,
            geometry,
        });
        self.current = Some(id);
        Ok(id)
    }

    /// Muestra la versión `id`, o el original si es `None`. No borra nada.
    pub fn set_current(&mut self, id: Option<u32>) -> Result<(), RecipeError> {
        if let Some(id) = id
            && !self.versions.iter().any(|v| v.id == id)
        {
            return Err(invalid(format!("no existe la versión {id}")));
        }
        self.current = id;
        Ok(())
    }
}

impl Geometry {
    /// Indica si no cambia nada la imagen.
    pub fn is_identity(&self) -> bool {
        self.rotate == 0 && !self.flip_horizontal && self.crop.is_none()
    }

    fn validate(&self) -> Result<(), String> {
        if !matches!(self.rotate, 0 | 90 | 180 | 270) {
            return Err(format!(
                "giro de {}°, solo se admiten 0, 90, 180 y 270",
                self.rotate
            ));
        }
        if let Some(c) = self.crop {
            let finite = [c.x, c.y, c.width, c.height].iter().all(|v| v.is_finite());
            let inside = c.x >= 0.0
                && c.y >= 0.0
                && c.width > 0.0
                && c.height > 0.0
                && c.x + c.width <= 1.0 + CROP_EPSILON
                && c.y + c.height <= 1.0 + CROP_EPSILON;
            if !finite || !inside {
                return Err("el recorte sale de la imagen".into());
            }
        }
        Ok(())
    }

    /// Tamaño tras aplicar la geometría a una imagen de `width` × `height`
    /// ya girada según el EXIF.
    pub fn output_size(&self, width: u32, height: u32) -> (u32, u32) {
        let (w, h) = if self.swaps_dimensions() {
            (height, width)
        } else {
            (width, height)
        };
        match self.crop {
            Some(crop) => {
                let (_, _, cw, ch) = crop_rect(w, h, &crop);
                (cw, ch)
            }
            None => (w.max(1), h.max(1)),
        }
    }

    /// Indica si el giro intercambia el ancho y el alto.
    pub fn swaps_dimensions(&self) -> bool {
        matches!(self.rotate, 90 | 270)
    }

    /// Aplica el giro, el volteo y el recorte a una imagen ya girada según
    /// el EXIF. Un giro no válido se trata como 0.
    pub fn apply(&self, img: DynamicImage) -> DynamicImage {
        let img = match self.rotate {
            90 => img.rotate90(),
            180 => img.rotate180(),
            270 => img.rotate270(),
            _ => img,
        };
        let img = if self.flip_horizontal {
            img.fliph()
        } else {
            img
        };
        match self.crop {
            Some(crop) => {
                let (x, y, w, h) = crop_rect(img.width(), img.height(), &crop);
                img.crop_imm(x, y, w, h)
            }
            None => img,
        }
    }
}

/// Rectángulo en píxeles (x, y, ancho, alto) de un recorte sobre una imagen
/// de `width` × `height`. Los bordes se redondean al píxel más cercano, se
/// ajustan a la imagen y el resultado mide al menos 1 × 1.
pub fn crop_rect(width: u32, height: u32, crop: &Crop) -> (u32, u32, u32, u32) {
    let (x0, x1) = span(width, crop.x, crop.width);
    let (y0, y1) = span(height, crop.y, crop.height);
    (x0, y0, x1 - x0, y1 - y0)
}

/// Inicio y fin en píxeles de un tramo `[start, start + len]` en fracciones
/// de `size`, con al menos un píxel.
fn span(size: u32, start: f64, len: f64) -> (u32, u32) {
    let size = size.max(1);
    // Un valor no finito (solo posible en una receta sin validar) se trata
    // como el recorte más amplio.
    let start = if start.is_finite() { start } else { 0.0 };
    let len = if len.is_finite() { len } else { 1.0 };
    // `size` cabe en un f64 sin pérdida y el resultado está en 0..=size.
    let to_px = |f: f64| (f.clamp(0.0, 1.0) * f64::from(size)).round() as u32;
    let start_px = to_px(start).min(size - 1);
    let end_px = to_px(start + len).clamp(start_px + 1, size);
    (start_px, end_px)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    const ROJO: Rgb<u8> = Rgb([255, 0, 0]);
    const VERDE: Rgb<u8> = Rgb([0, 255, 0]);
    const AZUL: Rgb<u8> = Rgb([0, 0, 255]);
    const BLANCO: Rgb<u8> = Rgb([255, 255, 255]);

    /// Imagen de `w` × `h` en cuatro cuadrantes: rojo arriba a la izquierda,
    /// verde arriba a la derecha, azul abajo a la izquierda y blanco abajo a
    /// la derecha.
    fn cuadrantes(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
            match (x < w / 2, y < h / 2) {
                (true, true) => ROJO,
                (false, true) => VERDE,
                (true, false) => AZUL,
                (false, false) => BLANCO,
            }
        }))
    }

    /// Colores del centro de cada cuadrante: arriba izquierda, arriba
    /// derecha, abajo izquierda y abajo derecha.
    fn esquinas(img: &DynamicImage) -> [Rgb<u8>; 4] {
        let img = img.to_rgb8();
        let (w, h) = img.dimensions();
        let at = |x: u32, y: u32| *img.get_pixel(x, y);
        [
            at(w / 4, h / 4),
            at(w * 3 / 4, h / 4),
            at(w / 4, h * 3 / 4),
            at(w * 3 / 4, h * 3 / 4),
        ]
    }

    fn original() -> OriginalRef {
        OriginalRef {
            hash: "9f2c".into(),
            width: 6000,
            height: 4000,
        }
    }

    fn crop(x: f64, y: f64, width: f64, height: f64) -> Option<Crop> {
        Some(Crop {
            x,
            y,
            width,
            height,
        })
    }

    #[test]
    fn la_identidad_no_cambia_nada() {
        let img = cuadrantes(40, 20);
        let g = Geometry::default();
        assert!(g.is_identity());
        assert_eq!(g.apply(img.clone()), img);
        assert_eq!(g.output_size(40, 20), (40, 20));
    }

    #[test]
    fn gira_en_sentido_horario() {
        let img = cuadrantes(40, 20);
        let gira = |rotate| {
            let g = Geometry {
                rotate,
                ..Geometry::default()
            };
            let out = g.apply(img.clone());
            assert_eq!((out.width(), out.height()), g.output_size(40, 20));
            esquinas(&out)
        };
        assert_eq!(gira(90), [AZUL, ROJO, BLANCO, VERDE]);
        assert_eq!(gira(180), [BLANCO, AZUL, VERDE, ROJO]);
        assert_eq!(gira(270), [VERDE, BLANCO, ROJO, AZUL]);
        assert_eq!(
            Geometry {
                rotate: 90,
                ..Geometry::default()
            }
            .output_size(40, 20),
            (20, 40)
        );
    }

    #[test]
    fn voltea_despues_de_girar() {
        let img = cuadrantes(40, 20);
        let g = Geometry {
            rotate: 90,
            flip_horizontal: true,
            crop: None,
        };
        // Girada: [AZUL, ROJO, BLANCO, VERDE]; volteada, izquierda y derecha
        // se cambian.
        assert_eq!(esquinas(&g.apply(img)), [ROJO, AZUL, VERDE, BLANCO]);
    }

    #[test]
    fn recorta_sobre_la_imagen_ya_girada() {
        let img = cuadrantes(40, 20);
        // Tras girar 90°, la mitad de arriba es azul (izquierda) y roja
        // (derecha). Recortar el cuarto de arriba a la derecha deja solo rojo.
        let g = Geometry {
            rotate: 90,
            flip_horizontal: false,
            crop: crop(0.5, 0.0, 0.5, 0.5),
        };
        let out = g.apply(img);
        assert_eq!((out.width(), out.height()), (10, 20));
        assert_eq!(g.output_size(40, 20), (10, 20));
        assert!(out.to_rgb8().pixels().all(|p| *p == ROJO));
    }

    #[test]
    fn el_resultado_no_depende_de_la_resolucion() {
        let g = Geometry {
            rotate: 270,
            flip_horizontal: true,
            crop: crop(0.25, 0.25, 0.5, 0.5),
        };
        let grande = g.apply(cuadrantes(4000, 2000));
        let pequena = g.apply(cuadrantes(400, 200));
        assert_eq!(esquinas(&grande), esquinas(&pequena));
        assert_eq!((grande.width(), grande.height()), (1000, 2000));
        assert_eq!((pequena.width(), pequena.height()), (100, 200));
    }

    #[test]
    fn el_recorte_se_ajusta_a_la_imagen_y_mide_al_menos_un_pixel() {
        let c = |x, y, w, h| Crop {
            x,
            y,
            width: w,
            height: h,
        };
        assert_eq!(crop_rect(100, 50, &c(0.0, 0.0, 1.0, 1.0)), (0, 0, 100, 50));
        assert_eq!(crop_rect(3, 3, &c(0.1, 0.1, 0.01, 0.01)), (0, 0, 1, 1));
        assert_eq!(crop_rect(10, 10, &c(0.99, 0.99, 0.5, 0.5)), (9, 9, 1, 1));
        assert_eq!(
            crop_rect(10, 10, &c(f64::NAN, 0.0, 1.0, 1.0)),
            (0, 0, 10, 10)
        );
        assert_eq!(crop_rect(0, 0, &c(0.0, 0.0, 1.0, 1.0)), (0, 0, 1, 1));
    }

    #[test]
    fn lee_y_escribe_el_formato_de_la_spec() {
        let json = br#"{
            "format": 1,
            "original": { "hash": "9f2c", "width": 6000, "height": 4000 },
            "current": 2,
            "versions": [
                { "id": 1, "created_at": 1762190400,
                  "geometry": { "rotate": 90, "flip_horizontal": false, "crop": null } },
                { "id": 2, "created_at": 1762194000,
                  "geometry": { "rotate": 90, "flip_horizontal": false,
                                "crop": { "x": 0.1, "y": 0.05, "width": 0.8, "height": 0.9 } } }
            ]
        }"#;
        let recipe = Recipe::parse(json).unwrap();
        assert!(recipe.is_writable());
        assert_eq!(recipe.current_geometry().crop, crop(0.1, 0.05, 0.8, 0.9));
        let again = Recipe::parse(&recipe.to_json().unwrap()).unwrap();
        assert_eq!(again, recipe);
    }

    #[test]
    fn las_versiones_se_acumulan_y_se_puede_volver_al_original() {
        let mut recipe = Recipe::new(original());
        assert!(recipe.current_geometry().is_identity());
        let giro = Geometry {
            rotate: 90,
            ..Geometry::default()
        };
        let recorte = Geometry {
            crop: crop(0.0, 0.0, 0.5, 0.5),
            ..giro
        };
        assert_eq!(recipe.add_version(giro, 10).unwrap(), 1);
        assert_eq!(recipe.add_version(recorte, 20).unwrap(), 2);
        assert_eq!(recipe.current_geometry(), recorte);

        recipe.set_current(None).unwrap();
        assert!(recipe.current_geometry().is_identity());
        assert_eq!(recipe.versions.len(), 2);

        recipe.set_current(Some(2)).unwrap();
        assert_eq!(recipe.current_geometry(), recorte);
        assert!(recipe.set_current(Some(7)).is_err());
        assert_eq!(recipe.current, Some(2));

        // Al volver a una versión vieja y guardar, el id sigue subiendo.
        recipe.set_current(Some(1)).unwrap();
        assert_eq!(recipe.add_version(giro, 30).unwrap(), 3);
    }

    #[test]
    fn rechaza_recetas_danadas() {
        let base = || Recipe {
            format: 1,
            original: original(),
            current: Some(1),
            versions: vec![Version {
                id: 1,
                created_at: 0,
                geometry: Geometry::default(),
            }],
        };
        assert!(base().validate().is_ok());

        let mut r = base();
        r.current = Some(2);
        assert!(r.validate().is_err());

        let mut r = base();
        r.versions.push(r.versions[0].clone());
        assert!(r.validate().is_err());

        let mut r = base();
        r.versions[0].geometry.rotate = 45;
        assert!(r.validate().is_err());

        for c in [
            crop(0.5, 0.0, 0.6, 1.0),
            crop(-0.1, 0.0, 0.5, 0.5),
            crop(0.0, 0.0, 0.0, 0.5),
            crop(f64::INFINITY, 0.0, 0.5, 0.5),
        ] {
            let mut r = base();
            r.versions[0].geometry.crop = c;
            assert!(r.validate().is_err(), "{c:?}");
        }

        let mut r = base();
        r.original.hash = "../../etc".into();
        assert!(r.validate().is_err());

        let mut r = base();
        r.format = 0;
        assert!(r.validate().is_err());

        assert!(Recipe::parse(b"{ roto").is_err());
        assert!(Recipe::parse(b"{}").is_err());
        assert!(
            base()
                .add_version(
                    Geometry {
                        rotate: 45,
                        ..Geometry::default()
                    },
                    0
                )
                .is_err()
        );
    }

    #[test]
    fn una_receta_de_un_formato_futuro_se_lee_pero_no_se_escribe() {
        let json = br#"{
            "format": 2,
            "original": { "hash": "ab", "width": 10, "height": 10 },
            "current": 1,
            "versions": [
                { "id": 1, "created_at": 0,
                  "geometry": { "rotate": 180, "crop": null },
                  "adjustments": { "exposure": 0.3 } }
            ]
        }"#;
        let mut recipe = Recipe::parse(json).unwrap();
        assert!(!recipe.is_writable());
        assert_eq!(recipe.current_geometry().rotate, 180);
        assert!(recipe.add_version(Geometry::default(), 1).is_err());
    }
}
