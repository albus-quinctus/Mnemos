-- Favoritos, archivo y título del día (spec 006). Solo cambia la base de
-- datos: nada se escribe en los archivos de imagen.

-- 1 si la foto está en Favoritos.
ALTER TABLE photos ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0
    CHECK (favorite IN (0, 1));

-- Cuándo se archivó (segundos Unix, UTC). NULL: está en la línea de tiempo.
ALTER TABLE photos ADD COLUMN archived_at INTEGER;

-- Un índice parcial por vista, con el mismo orden que `photos_timeline`, para
-- que Fotos, Favoritos y Archivo paginen igual de rápido. La búsqueda incluye
-- las archivadas y sigue usando `photos_timeline`. Las consultas tienen que
-- repetir la condición del índice para que SQLite lo use.
CREATE INDEX photos_timeline_fotos
    ON photos (coalesce(taken_at, modified_at) DESC, id DESC)
    WHERE archived_at IS NULL;
CREATE INDEX photos_timeline_favoritos
    ON photos (coalesce(taken_at, modified_at) DESC, id DESC)
    WHERE favorite = 1;
CREATE INDEX photos_timeline_archivo
    ON photos (coalesce(taken_at, modified_at) DESC, id DESC)
    WHERE archived_at IS NOT NULL;

-- Título libre de un día, con la fecha en hora local tal como agrupa la
-- cuadrícula ("2024-10-12").
CREATE TABLE day_titles (
    day         TEXT    PRIMARY KEY
                CHECK (day GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    title       TEXT    NOT NULL CHECK (length(title) BETWEEN 1 AND 100),
    updated_at  INTEGER NOT NULL
);
