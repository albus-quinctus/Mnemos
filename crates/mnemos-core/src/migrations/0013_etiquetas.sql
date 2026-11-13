-- Etiquetas (spec 016). Las tablas `tags` y `photo_tags` y la columna `tags`
-- del índice de búsqueda existen desde la migración 0001. Solo cambia la base
-- de datos: nada se escribe en los archivos de imagen.

-- Las fotos de una etiqueta (vista, recuento y portada) se buscan por
-- etiqueta; la clave primaria de `photo_tags` empieza por la foto.
CREATE INDEX photo_tags_tag ON photo_tags (tag_id, photo_id);

-- La columna `tags` del índice de búsqueda lleva los nombres de las etiquetas
-- de cada foto separados por espacios. Hasta ahora nadie la escribía: se
-- rehace a partir de `photo_tags` por si alguna fila no coincide.
UPDATE photo_search
   SET tags = coalesce((SELECT group_concat(t.name, ' ')
                          FROM photo_tags pt JOIN tags t ON t.id = pt.tag_id
                         WHERE pt.photo_id = photo_search.rowid), '')
 WHERE tags <> ''
    OR rowid IN (SELECT photo_id FROM photo_tags);
