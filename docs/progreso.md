# Progreso

> Lo primero que se lee en cada sesión. Mantener corto: estado, siguiente paso y pendientes.

## Estado
Fase **F1 Biblioteca y navegación**, en curso.
- F0 cerrada el 2026-10-25 (PR #2).
- Estilo visual decidido (decisión 003): cuadrícula estilo Galería y visor oscuro.
- Spec 001 (importar e indexar) implementada en núcleo y CLI: `mnemos index` y `mnemos import`. Copias verificadas con BLAKE3, sin tocar el origen, en `Originales/AAAA/MM/` según la fecha de modificación.

## Siguiente paso
1. Spec 002 (`docs/specs/002-miniaturas-cuadricula-visor.md`) hecha: miniaturas, cuadrícula, visor, abrir o crear biblioteca, importar desde la aplicación y rendimiento con 10 000 fotos (`docs/rendimiento.md`). Todos sus criterios de aceptación tienen prueba. La tarea 14 (HEIC y AVIF) queda aplazada; la prueba de `libheif` está en la decisión 004. Spec 003, búsqueda básica (`docs/specs/003-busqueda-basica.md`), hecha: campo de búsqueda con espera de 200 ms, resultados en la misma cuadrícula, Esc para volver y visor dentro de los resultados (25 ms en el peor caso con 10 000 fotos). Con ella F1 queda completa. Siguiente: cerrar F1 y elegir la primera spec de F2 en `docs/roadmap.md`.
2. Spec 004, fecha de captura (`docs/specs/004-fecha-de-captura.md`): EXIF, nombre del archivo y `.json` de Google Takeout. Salió al importar un Takeout real (1401 fotos, todas en `Originales/2026/10`). Para arreglar una biblioteca ya importada: `mnemos index` y volver a importar la carpeta de Takeout.
3. Spec 005, versión portátil (`docs/specs/005-version-portatil.md`): modo portátil en la app y zip con los tres ejecutables generado por `.github/workflows/portatil.yml`. Falta probarla en Windows y macOS, y pasar el HDD a exFAT (`docs/guias/pasar-el-disco-a-exfat.md`).
4. F2 empieza con la spec 006, selección múltiple, favoritos, archivo y título del día (`docs/specs/006-seleccion-favoritos-archivo.md`), en curso: hechas la tarea 1 (migración 0007 con `favorite`, `archived_at`, `day_titles` e índices parciales por vista) y la 2 (`Library::set_favorite` y `set_archived`, en una transacción y devolviendo los ids que cambian para «Deshacer»; 35 ms con 10 000 fotos). Tarea 3 hecha: `View` (Fotos sin archivadas, Favoritos, Archivo) con `view_months` y `view_page`, cada una con su índice parcial; la búsqueda incluye las archivadas y `GridPhoto` lleva `favorite` y `archived`. Tarea 4 hecha: `Scope` (vista o búsqueda) con `select_all`, `select_day` y `select_range`, en el orden de la cuadrícula y sin cargar páginas. Siguiente: tarea 5, títulos de día. Va antes que álbumes porque álbumes, etiquetas y papelera dependen de la selección. F1 sigue sin cerrar formalmente (último punto).
5. Spec 007, metadatos de Google Takeout (`docs/specs/007-metadatos-takeout.md`), en curso: el núcleo ya guarda descripción, ubicación, favorita, archivada y álbumes del `.json` al importar y al reimportar (migración 0008). Falta contrastarlo con el Takeout real (tarea 1). Para aplicarlo a la biblioteca del HDD: volver a importar la carpeta del Takeout.
6. Spec 008, exportar una foto desde el visor (`docs/specs/008-exportar.md`): tamaño original (copia verificada) o reducido (JPEG de 2048 px sin metadatos), sin sobrescribir nada y nunca dentro de la biblioteca. Falta probarlo en la app real. Exportar varias fotos espera a la selección de la spec 006.
7. Spec 010, duplicados exactos (`docs/specs/010-duplicados-exactos.md`): hechas las tareas 1 a 3 (consulta en el núcleo, `mnemos duplicados` y comandos de Tauri). Solo enseña las fotos repetidas, no borra nada. Falta la vista en la aplicación, que espera a la navegación lateral de la spec 006.
8. Probar con 10 000 fotos reales desde un HDD externo y medir indexado, miniaturas y visor dentro de Tauri (pendiente en `docs/rendimiento.md`).

## Pendientes / ideas aparcadas
- Revisar e instalar las skills de `docs/skills.md`.
- Crear `tests/fixtures/` con imágenes de ejemplo sin datos personales.
- Decidir qué hacer con las fotos desaparecidas (hoy solo se cuentan).
- Revelado de RAW (F3); hoy solo se usa la vista previa incrustada.
- Caché en disco de vistas grandes para el visor, si la precarga en memoria se queda corta.
- Miniaturas de JPEG más rápidas: hechas el 2026-10-27 con decodificación reducida (de 364 a 132 ms en una foto de 24 MP, ver `docs/rendimiento.md`). Falta medirlas con fotos reales desde el HDD con el ejemplo `medir_miniaturas`.
- Probar el extractor de vistas previas con RAW reales de cada marca (Sony ARW, Pentax PEF, Samsung SRW, Olympus ORF antiguos). Solo se ha probado con archivos sintéticos y con las muestras recortadas de exiftool.
- `pnpm audit` avisa de 3 fallos leves en `@sveltejs/kit` y `cookie` que solo afectan al servidor de SvelteKit (la app es estática). Actualizar SvelteKit cuando haya versión corregida.
