# 001 · Stack y licencia

- Fecha: 2026-10-23
- Estado: **aceptada** (2026-10-23)

## Contexto
MNEMOS debe ejecutarse desde un disco externo en Windows, macOS y Linux, sin instalación y con poco consumo.

## Decisión
- Tauri 2 + núcleo Rust + Svelte 5/TypeScript + SQLite (FTS5).
- Licencia GPL-3.0-or-later.

## Alternativas consideradas
- Electron: plan B si WebKitGTK da problemas en Linux. Ejecutables más grandes.
- Licencia Apache-2.0: más permisiva; permite forks cerrados.

## Consecuencias
- Ejecutables pequeños y núcleo seguro en memoria.
- En Linux depende de WebKitGTK del sistema; hay que probar el AppImage en varias distribuciones.
