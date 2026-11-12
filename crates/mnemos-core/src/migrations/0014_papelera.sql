-- Papelera (spec 015). Enviar a la Papelera solo pone la fecha: el archivo
-- sigue en su sitio. Solo «Eliminar para siempre» lo borra (regla 1).

-- Cuándo se envió a la Papelera (segundos Unix, UTC). NULL: no está en ella.
ALTER TABLE photos ADD COLUMN trashed_at INTEGER;

-- Las vistas de la línea de tiempo dejan fuera la Papelera. Se rehacen los
-- índices parciales de la migración 0007 con la condición nueva; las
-- consultas tienen que repetirla para que SQLite los use.
DROP INDEX photos_timeline_fotos;
DROP INDEX photos_timeline_favoritos;
DROP INDEX photos_timeline_archivo;
CREATE INDEX photos_timeline_fotos
    ON photos (coalesce(taken_at, modified_at) DESC, id DESC)
    WHERE archived_at IS NULL AND trashed_at IS NULL;
CREATE INDEX photos_timeline_favoritos
    ON photos (coalesce(taken_at, modified_at) DESC, id DESC)
    WHERE favorite = 1 AND trashed_at IS NULL;
CREATE INDEX photos_timeline_archivo
    ON photos (coalesce(taken_at, modified_at) DESC, id DESC)
    WHERE archived_at IS NOT NULL AND trashed_at IS NULL;

-- La Papelera se ordena por la fecha de envío, de la más reciente a la más
-- antigua; también sirve para buscar lo que lleva 30 días.
CREATE INDEX photos_papelera
    ON photos (trashed_at DESC, id DESC)
    WHERE trashed_at IS NOT NULL;

-- Datos de la biblioteca que viajan con ella. `ultima_apertura` (segundos
-- Unix) protege el vaciado automático de un reloj adelantado.
CREATE TABLE library_meta (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);
