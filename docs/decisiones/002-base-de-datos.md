# 002 · Base de datos y migraciones

- Fecha: 2026-10-23
- Estado: aceptada

## Contexto
La biblioteca vive en un disco externo que puede desconectarse en cualquier momento y que suele estar formateado en exFAT.

## Decisión
- SQLite incluido en el ejecutable (`rusqlite` con `bundled`), sin depender de la versión del sistema.
- Migraciones en SQL dentro de `crates/mnemos-core/src/migrations/`, numeradas, aplicadas en transacción y registradas en `PRAGMA user_version`. Una migración publicada no se edita nunca.
- `journal_mode = DELETE` y `synchronous = FULL`: más lento que WAL, pero sin archivos `-wal`/`-shm` y más seguro ante desconexiones.
- Claves ajenas activadas en cada conexión.
- Si la base de datos tiene un esquema más nuevo que el que entiende la aplicación, se rechaza abrirla en vez de arriesgarse a dañarla.
- Búsqueda con FTS5 (`unicode61 remove_diacritics 2`) para que "malaga" encuentre "Málaga".

## Alternativas consideradas
- WAL: mejor rendimiento con varios lectores, pero deja archivos auxiliares y se comporta peor en discos extraíbles.
- Crate de migraciones (`refinery`, `rusqlite_migration`): innecesario para el volumen actual.

## Consecuencias
- Si el rendimiento de escritura se queda corto al importar miles de fotos, se agrupan las inserciones en transacciones antes de plantear WAL.
