-- Fecha de captura (spec 004): de dónde salió `taken_at`. NULL significa que
-- aún no se ha buscado; 'ninguna', que se buscó y no hay.
ALTER TABLE photos ADD COLUMN taken_source TEXT;
