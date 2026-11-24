-- Valoración, cámara y objetivo (spec 024). Solo cambia la base de datos:
-- nada se escribe en los archivos ni se crean `.xmp`.

-- Valoración de 0 (sin estrellas) a 5.
ALTER TABLE photos ADD COLUMN rating INTEGER NOT NULL DEFAULT 0
    CHECK (rating BETWEEN 0 AND 5);

-- De dónde salió: 'archivo' (XMP o EXIF), 'manual' (puesta en MNEMOS) o NULL.
-- La manual no se pisa nunca.
ALTER TABLE photos ADD COLUMN rating_source TEXT
    CHECK (rating_source IN ('archivo', 'manual'));

-- Cámara (marca y modelo) y objetivo del EXIF, para los filtros.
ALTER TABLE photos ADD COLUMN camera TEXT;
ALTER TABLE photos ADD COLUMN lens TEXT;

-- Cuándo se leyeron el XMP, la valoración EXIF y la cámara (segundos Unix,
-- UTC). NULL: todavía no. Se leen una sola vez, como el `.json` de Takeout.
ALTER TABLE photos ADD COLUMN metadata_at INTEGER;

CREATE INDEX photos_rating ON photos (rating) WHERE rating > 0;
CREATE INDEX photos_camera ON photos (camera, lens) WHERE camera IS NOT NULL;
-- Las fotos que faltan por leer.
CREATE INDEX photos_metadata_pending ON photos (id) WHERE metadata_at IS NULL;
