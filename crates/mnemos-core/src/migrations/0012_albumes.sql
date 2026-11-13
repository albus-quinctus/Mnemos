-- Álbumes (spec 014). Las tablas `albums` y `album_photos` vienen de la 0001;
-- solo cambia la base de datos: nada se escribe en los archivos de imagen.

-- Última vez que se creó, renombró o cambió el álbum (segundos Unix, UTC),
-- para ordenar la pantalla Álbumes. Los álbumes que ya existen (los del
-- Takeout) toman su fecha de creación.
ALTER TABLE albums ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE albums SET updated_at = created_at;

-- La clave primaria de `album_photos` empieza por `album_id`; este índice
-- sirve para lo contrario: en qué álbumes está una foto y borrar fotos sin
-- recorrer la tabla.
CREATE INDEX album_photos_photo ON album_photos (photo_id);
