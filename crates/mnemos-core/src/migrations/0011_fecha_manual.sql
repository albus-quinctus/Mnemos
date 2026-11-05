-- Fecha de captura a mano (spec 011). Cuando `taken_source` es 'manual',
-- estas columnas guardan la fecha automática que había, para poder volver a
-- ella. En el resto de las fotos son NULL. Nada se escribe en los archivos.
ALTER TABLE photos ADD COLUMN taken_auto_at INTEGER;
ALTER TABLE photos ADD COLUMN taken_auto_source TEXT;
