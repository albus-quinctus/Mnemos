# MNEMOS

Gestor de fotos portátil, local y de código abierto. Funciona desde un disco externo en Windows, macOS y Linux.

## Stack
- Tauri 2 (aplicación de escritorio), núcleo en Rust, interfaz en Svelte 5 + TypeScript, SQLite con FTS5.
- Monorepo: `apps/desktop` (Tauri + Svelte), `crates/mnemos-core`, `crates/mnemos-imaging`, `crates/mnemos-cli`.

## Comandos
- Rust: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`
- Interfaz: `pnpm install`, `pnpm check`, `pnpm test`, `pnpm tauri dev`
- Dependencias: `cargo deny check`, `pnpm audit`

## Reglas que no se rompen
1. **Nunca modificar, mover ni borrar un original** de la biblioteca. Las ediciones son recetas JSON en `.mnemos/ediciones/`.
2. **Solo rutas relativas** a la raíz de la biblioteca. Nada de rutas absolutas en la base de datos.
3. Las pruebas usan `tests/fixtures/`, nunca una biblioteca real.
4. Sin servicios en la nube, cuentas ni telemetría.
5. Sin `unsafe` en Rust sin una nota `// SAFETY:` y una decisión en `docs/decisiones/`.

## Cómo trabajar
- Empieza leyendo `docs/progreso.md`. No leas más documentos de los que la tarea necesite.
- Una tarea por sesión. Si surge algo nuevo, anótalo en `docs/progreso.md` y no lo hagas ahora.
- Cada funcionalidad tiene su especificación en `docs/specs/`. Si no existe, escríbela antes de programar.
- Al terminar: pruebas en verde, actualiza `docs/progreso.md` y abre un PR pequeño contra `develop` (nunca contra `main`).
- Commits y títulos de PR en [Conventional Commits](https://www.conventionalcommits.org/es/v1.0.0/): `feat(core): añade indexado de carpetas`.
- Sin menciones a la IA en commits ni PR: nada de `Co-Authored-By` de asistentes ni pies "Generated with".
- Lee archivos por rangos y acota las búsquedas. Responde de forma breve.

## Más detalle (solo si hace falta)
`docs/workflow.md` · `docs/arquitectura.md` · `docs/roadmap.md` · `docs/skills.md` · `docs/decisiones/`
