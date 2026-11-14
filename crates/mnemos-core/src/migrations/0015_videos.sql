-- Vídeos (spec 019). Viven en la misma tabla que las fotos: favoritos,
-- archivo, álbumes, etiquetas y papelera los tratan igual. Las filas que ya
-- había son fotos: hasta ahora no se importaban vídeos.
ALTER TABLE photos ADD COLUMN media_kind TEXT NOT NULL DEFAULT 'foto'
    CHECK (media_kind IN ('foto', 'video'));

-- Duración en milisegundos, si el contenedor la dice. NULL en las fotos.
ALTER TABLE photos ADD COLUMN duration_ms INTEGER;
