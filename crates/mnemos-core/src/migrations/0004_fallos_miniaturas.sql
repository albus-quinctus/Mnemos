-- Fotos cuya miniatura no se pudo generar (formato sin soporte o imagen
-- dañada). Va por huella: si el archivo cambia, cambia la huella y se vuelve
-- a intentar. Las imágenes que se pueden leer nunca aparecen aquí.
CREATE TABLE thumbnail_failures (
    content_hash  TEXT    PRIMARY KEY,
    message       TEXT    NOT NULL,
    failed_at     INTEGER NOT NULL
);
