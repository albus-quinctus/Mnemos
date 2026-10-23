-- Esquema inicial de la biblioteca de MNEMOS.
-- Todas las rutas son relativas a la raíz de la biblioteca y usan '/'.
-- Las fechas son segundos desde la época Unix (UTC).

CREATE TABLE photos (
    id            INTEGER PRIMARY KEY,
    rel_path      TEXT    NOT NULL UNIQUE,
    size_bytes    INTEGER NOT NULL,
    modified_at   INTEGER NOT NULL,
    content_hash  TEXT,
    width         INTEGER,
    height        INTEGER,
    taken_at      INTEGER,
    imported_at   INTEGER NOT NULL
);

CREATE INDEX photos_taken_at ON photos (taken_at);
CREATE INDEX photos_content_hash ON photos (content_hash);

CREATE TABLE albums (
    id          INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE TABLE album_photos (
    album_id  INTEGER NOT NULL REFERENCES albums (id) ON DELETE CASCADE,
    photo_id  INTEGER NOT NULL REFERENCES photos (id) ON DELETE CASCADE,
    position  INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (album_id, photo_id)
);

CREATE TABLE tags (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE photo_tags (
    photo_id  INTEGER NOT NULL REFERENCES photos (id) ON DELETE CASCADE,
    tag_id    INTEGER NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (photo_id, tag_id)
);

-- Índice de búsqueda. rowid = photos.id. Se mantiene desde el núcleo.
CREATE VIRTUAL TABLE photo_search USING fts5 (
    rel_path,
    tags,
    tokenize = 'unicode61 remove_diacritics 2'
);
