# Progreso

> Lo primero que se lee en cada sesión. Mantener corto: estado, siguiente paso y pendientes.

## Estado
Fase **F1 Biblioteca y navegación**, en curso.
- F0 cerrada el 2026-10-25 (PR #2).
- Estilo visual decidido (decisión 003): cuadrícula estilo Galería y visor oscuro.
- Spec 001 (importar e indexar) implementada en núcleo y CLI: `mnemos index` y `mnemos import`. Copias verificadas con BLAKE3, sin tocar el origen, en `Originales/AAAA/MM/` según la fecha de modificación.

## Siguiente paso
1. Spec 002 (`docs/specs/002-miniaturas-cuadricula-visor.md`) en curso. Hechas: tareas 1 a 9 (decisión 004, RAW y orientación, miniaturas, vistas previas RAW, cola en segundo plano, `mnemos miniaturas`, protocolo `mnemos://`, consulta paginada por mes, estructura de la Galería y cuadrícula justificada y virtualizada, que ya genera las miniaturas al abrir la biblioteca). Siguiente: tarea 10, visor.
2. Probar con 10 000 fotos desde un HDD externo y medir tiempos de indexado.

## Pendientes / ideas aparcadas
- Revisar e instalar las skills de `docs/skills.md`.
- Crear `tests/fixtures/` con imágenes de ejemplo sin datos personales.
- Decidir qué hacer con las fotos desaparecidas (hoy solo se cuentan).
- Revelado de RAW (F3); hoy solo se usa la vista previa incrustada.
- Caché en disco de vistas grandes para el visor, si la precarga en memoria se queda corta.
- Miniaturas de JPEG más rápidas: hoy una foto de 24 MP tarda unos 300 ms en modo release porque se decodifica entera. Probar la decodificación reducida (escalado DCT 1/2, 1/4 y 1/8) en la tarea 15.
- Probar el extractor de vistas previas con RAW reales de cada marca (Sony ARW, Pentax PEF, Samsung SRW, Olympus ORF antiguos). Solo se ha probado con archivos sintéticos y con las muestras recortadas de exiftool.
- Empaquetado portátil (ejecutables en la raíz del disco) y prueba en exFAT.
- `pnpm audit` avisa de 3 fallos leves en `@sveltejs/kit` y `cookie` que solo afectan al servidor de SvelteKit (la app es estática). Actualizar SvelteKit cuando haya versión corregida.
