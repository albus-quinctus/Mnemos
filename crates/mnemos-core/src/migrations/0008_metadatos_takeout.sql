-- Metadatos de Google Takeout (spec 007). Solo cambia la base de datos: nada
-- se escribe en los archivos de imagen.

-- Descripción escrita en Google Fotos.
ALTER TABLE photos ADD COLUMN description TEXT;

-- Ubicación en grados (WGS84) y altitud en metros. NULL: sin ubicación.
ALTER TABLE photos ADD COLUMN latitude REAL
    CHECK (latitude BETWEEN -90 AND 90);
ALTER TABLE photos ADD COLUMN longitude REAL
    CHECK (longitude BETWEEN -180 AND 180);
ALTER TABLE photos ADD COLUMN altitude REAL;

-- Cuándo se aplicaron los metadatos del `.json` de Takeout (segundos Unix,
-- UTC). NULL: todavía no. Se aplican una sola vez para que volver a importar
-- no deshaga lo que el usuario cambie después en MNEMOS.
ALTER TABLE photos ADD COLUMN takeout_at INTEGER;

-- Los álbumes de Takeout se buscan por nombre al importar.
CREATE INDEX albums_name ON albums (name);
