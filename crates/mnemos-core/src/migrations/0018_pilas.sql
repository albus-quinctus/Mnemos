-- Pilas (spec 023). Solo cambia la base de datos: ningún archivo se mueve,
-- se renombra ni se borra.

CREATE TABLE stacks (
    id          INTEGER PRIMARY KEY,
    -- Foto que se enseña en la cuadrícula. La mantiene el núcleo: siempre es
    -- una foto de la pila.
    cover_id    INTEGER NOT NULL,
    kind        TEXT    NOT NULL
                CHECK (kind IN ('raw_jpeg', 'editada', 'rafaga', 'manual')),
    created_at  INTEGER NOT NULL
);

ALTER TABLE photos ADD COLUMN stack_id INTEGER
    REFERENCES stacks (id) ON DELETE SET NULL;
-- 1 si la foto está en una pila y no es su portada: las vistas la esconden
-- con `stack_hidden = 0`. Copia de `stack_id` y `cover_id` para que las
-- consultas de la cuadrícula no tengan que unir tablas.
ALTER TABLE photos ADD COLUMN stack_hidden INTEGER NOT NULL DEFAULT 0
    CHECK (stack_hidden IN (0, 1));
-- Cuándo la miró la agrupación automática (segundos Unix). NULL: pendiente.
ALTER TABLE photos ADD COLUMN stack_auto_at INTEGER;
-- 1 si el usuario la sacó de una pila o desapiló la suya: la agrupación
-- automática no la vuelve a apilar.
ALTER TABLE photos ADD COLUMN stack_manual INTEGER NOT NULL DEFAULT 0
    CHECK (stack_manual IN (0, 1));

CREATE INDEX photos_stack ON photos (stack_id) WHERE stack_id IS NOT NULL;
