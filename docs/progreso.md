# Progreso

> Lo primero que se lee en cada sesión. Mantener corto: estado, siguiente paso y pendientes.

## Estado
Fase **F1 Biblioteca y navegación**, en curso.
- F0 cerrada el 2026-10-25 (PR #2).
- Estilo visual decidido (decisión 003): cuadrícula estilo Galería y visor oscuro.
- Spec 001 (importar e indexar) implementada en núcleo y CLI: `mnemos index` y `mnemos import`. Copias verificadas con BLAKE3, sin tocar el origen, en `Originales/AAAA/MM/` según la fecha de modificación.

## Siguiente paso
1. Spec 002: miniaturas en `.mnemos/miniaturas/`, cuadrícula y visor en la aplicación, con importación y progreso desde la interfaz (plugin `dialog` para elegir carpetas).
2. Probar con 10 000 fotos desde un HDD externo y medir tiempos de indexado.

## Pendientes / ideas aparcadas
- Revisar e instalar las skills de `docs/skills.md`.
- Crear `tests/fixtures/` con imágenes de ejemplo sin datos personales.
- Decidir qué hacer con las fotos desaparecidas (hoy solo se cuentan).
- Empaquetado portátil (ejecutables en la raíz del disco) y prueba en exFAT.
