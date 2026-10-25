# Progreso

> Lo primero que se lee en cada sesión. Mantener corto: estado, siguiente paso y pendientes.

## Estado
Fase **F0 Fundaciones**, casi cerrada (2026-10-23):
- Monorepo: workspace Cargo (`crates/mnemos-core`, `mnemos-imaging`, `mnemos-cli`) y `apps/desktop` (Tauri 2 + SvelteKit/Svelte 5).
- Núcleo: crear/abrir biblioteca, rutas relativas validadas (`RelPath`), SQLite con migraciones por `user_version` y esquema v1 (fotos, álbumes, etiquetas, búsqueda FTS5).
- CLI: `mnemos init` y `mnemos info`.
- Escritorio: ventana que abre una biblioteca existente y muestra su resumen.
- CI: interfaz, Rust, cargo-deny y compilación en Windows, macOS y Linux.
- Licencia GPL-3.0-or-later (decisión 001 aceptada).

## Siguiente paso
1. Comprobar que la CI compila en los tres sistemas y probar `pnpm tauri dev` en un equipo real.
2. Proteger `main` y `develop` en GitHub.
3. Empezar F1 con `docs/specs/001-importar-carpeta.md` (importar e indexar fotos).

## Pendientes / ideas aparcadas
- Revisar e instalar las skills de `docs/skills.md`.
- Crear `tests/fixtures/` con imágenes de ejemplo sin datos personales.
- Elegir el selector de carpetas (plugin `dialog` de Tauri) para no escribir la ruta a mano.
- Empaquetado portátil (ejecutables en la raíz del disco) y prueba en exFAT.
