# Workflow de desarrollo

Versión completa: documento "Workflow de desarrollo de MNEMOS". Esto es el resumen operativo.

## Principios
1. Una tarea, una sesión. Entre tareas, `/clear` o sesión nueva.
2. El plan vive en archivos (`CLAUDE.md`, `docs/`), no en el chat.
3. Medir antes de optimizar (`npx ccusage@latest daily`, una vez por semana).
4. Mínimas skills: solo las de `docs/skills.md`.
5. Nada se instala sin leerlo.
6. Los originales son sagrados.

## Ciclo por funcionalidad
| Paso | Qué | Herramienta | Salida |
|---|---|---|---|
| 1. Especificar | Qué y por qué, casos límite, criterios de aceptación | brainstorming | `docs/specs/NNN-*.md` |
| 2. Planificar | Tareas de ≤1 h con archivos y pruebas | writing-plans | `docs/planes/NNN-*.md` (o lista de tareas en la spec si es pequeña) |
| 3. Implementar | Una tarea por sesión, rama `feat/NNN-*` | Claude Code + skills de stack | commits pequeños |
| 4. Verificar | fmt, clippy, tests, `pnpm check` | CI | todo en verde |
| 5. Revisar | Revisión del diff; seguridad si toca archivos o IPC | differential-review | correcciones |
| 6. Cerrar | Actualizar `docs/progreso.md`, PR, fusionar | `/clear` | estado al día |

Si una tarea pasa de ~30 mensajes, estaba mal dividida: vuelve al paso 2.

## Contexto
- Empezar leyendo solo `CLAUDE.md` y `docs/progreso.md`.
- Leer archivos por rangos; búsquedas acotadas.
- Subagentes solo para búsquedas amplias.
- `/compact` hacia el 60 % del contexto, indicando qué conservar.

## Rutina semanal
- Lunes: revisar `docs/progreso.md`, elegir 3-5 tareas, ajustar specs.
- Martes a jueves: ciclo por funcionalidad.
- Viernes: revisión de seguridad de lo fusionado, `ccusage`, actualizar hoja de ruta, podar `CLAUDE.md`.

## Ramas
- `main`: solo versiones publicadas del producto. Se actualiza desde `develop` al sacar una versión y se etiqueta (`v0.1.0`).
- `develop`: rama de integración. Todo el trabajo entra aquí por PR.
- Ramas de trabajo desde `develop`: `feat/`, `fix/`, `docs/`, `chore/`, `refactor/`, `test/`, `ci/`.

## Commits
Se sigue [Conventional Commits 1.0.0](https://www.conventionalcommits.org/es/v1.0.0/):

```
<tipo>(<ámbito opcional>): <descripción en imperativo y en minúscula>

<cuerpo opcional>

<pies opcionales, p. ej. BREAKING CHANGE: ...>
```

- Tipos: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
- Ámbitos: `core`, `imaging`, `cli`, `ui`, `db`, `deps`.
- Cambios incompatibles: `!` tras el tipo (`feat(db)!: ...`) o pie `BREAKING CHANGE:`.
- Ejemplos: `feat(core): añade indexado de carpetas`, `fix(ui): corrige el zoom del visor`.
- El título de cada PR sigue el mismo formato.
- Commits y PR sin menciones a herramientas de IA (sin `Co-Authored-By` de asistentes ni pies tipo "Generated with").
