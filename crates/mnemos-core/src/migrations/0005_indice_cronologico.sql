-- Orden de la cuadrícula: de lo más reciente a lo más antiguo, por fecha de
-- captura o, mientras no exista (F2), por fecha de modificación. El id
-- desempata para que la paginación sea estable.
CREATE INDEX photos_timeline ON photos (coalesce(taken_at, modified_at) DESC, id DESC);
