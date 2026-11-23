# Progreso

> Lo primero que se lee en cada sesión. Mantener corto: estado, siguiente paso y pendientes. El detalle de cada funcionalidad está en su spec (`docs/specs/`).

## Estado (23 nov 2026)
- **F0** cerrada el 2026-10-25 (PR #2).
- **F1** (0.1 alfa) con todas sus specs fusionadas (001 a 005). Falta su criterio de salida: 10 000 fotos reales fluidas desde el HDD, medidas dentro de Tauri (`docs/rendimiento.md`), y publicar la 0.1 alfa.
- **F2 y F3**: fusionadas las specs 006 a 021 (selección, Takeout, exportar, editor, duplicados exactos, Detalles y fecha a mano, acceso directo, Ajustes y F11, álbumes, papelera, etiquetas, exportar varias, aviso de ediciones compartidas, vídeos, copiar y descargar, Explorar fase A). Siguiente migración libre: 0016.

## Plan en curso: cimientos y funciones inspiradas en Immich
Orden acordado el 23 nov. Los números de migración están reservados para que las specs puedan avanzar en paralelo.

| Spec | Contenido | Migración | Estado |
|---|---|---|---|
| 022 | Cimientos: copias de la base, restaurar, hora local de captura, comprobar biblioteca | 0016, 0017 | Escribiendo la spec |
| 023 | Pilas (RAW+JPEG, ráfagas, «-editado») | 0018 | Espera: qué hacen las acciones sobre una pila |
| 024 | Filtros de búsqueda y estrellas (XMP de solo lectura) | 0019 | Escribiendo la spec; los filtros por fecha esperan a la hora local de la 022 |
| 025 | Live Photos y fotos con movimiento | 0020 | Espera la prueba de vídeos en Sway |
| 026 | Lugares sin conexión (Explorar fase B) y cola de trabajos | — | F4, después |
| 027 | Duplicados parecidos | — | F4, después |
| 028 | Diapositivas, carpetas y archivos grandes | — | F4, después |

Decisiones pendientes de Paulo: cuántas copias diarias de la base guardar (propuesta: 7) y si las acciones sobre una pila afectan a toda la pila o solo a la portada.

## Pruebas de Paulo pendientes (app real, HDD)
- Vídeos en Sway (spec 019, tarea 13): PR #135 (registro del error) y #136 (abrir con el programa del sistema), en borrador. Su resultado decide si hace falta una spec de `ffmpeg` antes de la 025.
- Explorar (spec 021, tarea 9).
- 10 000 fotos reales (cierre de F1).
- Copiar y descargar en Sway (spec 020, tarea 8).
- F11 y tema oscuro en Sway (013), exportar varias (017), editor (009), aviso de ediciones compartidas (018), papelera (015), etiquetas y álbumes (016, 014), acceso directo en Linux y Windows (012), metadatos del Takeout (007), versión portátil en Windows y macOS (005), AppImage con doble clic en Sway.

## Pendientes / ideas aparcadas
- Crear `tests/fixtures/` con imágenes de ejemplo sin datos personales.
- Decidir qué hacer con las fotos desaparecidas (hoy solo se cuentan).
- Revelado de RAW (F3); hoy solo se usa la vista previa incrustada.
- HEIC y AVIF (tarea 14 de la spec 002, decisión 004).
- Caché en disco de vistas grandes para el visor, si la precarga se queda corta (puede entrar en la 022 si la prueba de 10 000 fotos lo pide).
- Medir las miniaturas JPEG con fotos reales del HDD (`medir_miniaturas`).
- Probar el extractor de vistas previas con RAW reales de cada marca (ARW, PEF, SRW, ORF antiguos).
- `pnpm audit`: 3 fallos leves de `@sveltejs/kit` y `cookie` que solo afectan al servidor; actualizar SvelteKit cuando haya versión corregida.
- Revisar e instalar las skills de `docs/skills.md`.
- Después de la 1.0: mapa sin conexión, panorámicas 360°, modo solo lectura, etiquetas jerárquicas, aprendizaje automático local, prueba en Android.
