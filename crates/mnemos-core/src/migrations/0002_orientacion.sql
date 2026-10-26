-- Orientación EXIF (1 a 8) de cada foto. NULL significa que aún no se ha
-- leído; el indexado la rellena sin volver a calcular la huella.
ALTER TABLE photos ADD COLUMN orientation INTEGER;
