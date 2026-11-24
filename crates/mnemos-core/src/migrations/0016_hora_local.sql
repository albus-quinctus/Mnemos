-- Hora local de captura (spec 022). La cuadrícula, los días, los meses y la
-- búsqueda por fecha agrupan por la hora que marcaba la cámara, no por la zona
-- del equipo que abre la biblioteca.

-- Hora de pared de la captura en segundos, leída como si fuera UTC: la foto de
-- las 23:30 del 14 de julio de 2024 vale unixepoch('2024-07-14 23:30:00') en
-- cualquier equipo. `strftime(..., taken_local, 'unixepoch')` da su día y su
-- hora sin 'localtime'. Las fotos sin fecha de captura llevan aquí la fecha de
-- modificación, pasada a hora local con la zona del equipo que la indexó, para
-- ordenar y agrupar por una sola columna; `taken_source` sigue diciendo que no
-- es fecha de captura.
ALTER TABLE photos ADD COLUMN taken_local INTEGER;
-- Desfase respecto a UTC en minutos (EXIF `OffsetTimeOriginal`, fecha de los
-- vídeos de Apple), o NULL si no se sabe.
ALTER TABLE photos ADD COLUMN taken_offset INTEGER;
-- Lo mismo para la fecha automática que se guarda mientras hay fecha a mano
-- (spec 011), para volver a ella al quitarla.
ALTER TABLE photos ADD COLUMN taken_auto_local INTEGER;
ALTER TABLE photos ADD COLUMN taken_auto_offset INTEGER;

-- 1 mientras la hora local sea la de reserva que pone esta migración y falte
-- leer del original la hora de la cámara y su desfase. `mnemos index` la
-- afina y la pone a 0; las fotos nuevas ya entran con la buena.
ALTER TABLE photos ADD COLUMN taken_local_pending INTEGER NOT NULL DEFAULT 0;

-- Relleno con lo que se veía hasta ahora (la zona del equipo que migra): el
-- primer día no se mueve nada.
UPDATE photos SET
    taken_local = unixepoch(coalesce(taken_at, modified_at), 'unixepoch', 'localtime'),
    taken_auto_local = unixepoch(taken_auto_at, 'unixepoch', 'localtime'),
    taken_local_pending = 1;

-- Quien escriba `taken_at` o `modified_at` sin dar la hora local (una fila
-- nueva sin ella, o un UPDATE que no la toca) recibe la de reserva: la del
-- equipo, sin desfase. El código del núcleo la da siempre; esto mantiene la
-- columna rellena para cualquier otra escritura.
CREATE TRIGGER photos_hora_local_alta AFTER INSERT ON photos
WHEN NEW.taken_local IS NULL
BEGIN
    UPDATE photos SET
        taken_local = unixepoch(coalesce(NEW.taken_at, NEW.modified_at), 'unixepoch', 'localtime'),
        taken_offset = NULL
    WHERE id = NEW.id;
END;

CREATE TRIGGER photos_hora_local_cambio AFTER UPDATE OF taken_at, modified_at ON photos
WHEN NEW.taken_local IS OLD.taken_local AND NEW.taken_offset IS OLD.taken_offset
    AND (NEW.taken_at IS NOT OLD.taken_at
         OR (NEW.taken_at IS NULL AND NEW.modified_at IS NOT OLD.modified_at))
BEGIN
    UPDATE photos SET
        taken_local = unixepoch(coalesce(NEW.taken_at, NEW.modified_at), 'unixepoch', 'localtime'),
        taken_offset = NULL
    WHERE id = NEW.id;
END;

CREATE TRIGGER photos_hora_local_auto AFTER UPDATE OF taken_auto_at ON photos
WHEN NEW.taken_auto_local IS OLD.taken_auto_local
    AND NEW.taken_auto_offset IS OLD.taken_auto_offset
    AND NEW.taken_auto_at IS NOT OLD.taken_auto_at
BEGIN
    UPDATE photos SET
        taken_auto_local = unixepoch(NEW.taken_auto_at, 'unixepoch', 'localtime'),
        taken_auto_offset = NULL
    WHERE id = NEW.id;
END;

-- La línea de tiempo ordena por la hora local. Se rehacen los índices de las
-- migraciones 0005 y 0014 con las mismas condiciones parciales.
DROP INDEX photos_timeline;
DROP INDEX photos_timeline_fotos;
DROP INDEX photos_timeline_favoritos;
DROP INDEX photos_timeline_archivo;
CREATE INDEX photos_timeline ON photos (taken_local DESC, id DESC);
CREATE INDEX photos_timeline_fotos
    ON photos (taken_local DESC, id DESC)
    WHERE archived_at IS NULL AND trashed_at IS NULL;
CREATE INDEX photos_timeline_favoritos
    ON photos (taken_local DESC, id DESC)
    WHERE favorite = 1 AND trashed_at IS NULL;
CREATE INDEX photos_timeline_archivo
    ON photos (taken_local DESC, id DESC)
    WHERE archived_at IS NOT NULL AND trashed_at IS NULL;
