-- Edición no destructiva (spec 009). Versión de la receta que se muestra, o
-- NULL si se ve el original. Es solo una caché para la cuadrícula y el
-- protocolo mnemos://: manda la receta de `.mnemos/ediciones/`, y el indexado
-- vuelve a calcular esta columna a partir de ellas. Va detrás de la 0011 en la
-- lista de `db.rs`, porque las migraciones se aplican por su posición.
ALTER TABLE photos ADD COLUMN edit_version INTEGER
    CHECK (edit_version IS NULL OR edit_version > 0);
