# Skills de IA del proyecto

Evaluadas el 2026-10-23 leyendo solo sus README. **Ninguna está instalada todavía**: cada una se revisa con la lista de comprobación antes de copiarla a `.claude/skills/`.

## Lista de comprobación antes de instalar
- [ ] Leer `SKILL.md` completo y todos los archivos de su carpeta.
- [ ] ¿Ejecuta scripts, `npx`, `curl` o descargas? ¿Qué hacen exactamente?
- [ ] ¿Instala hooks o toca `settings.json`? Si es así, se descarta salvo decisión explícita.
- [ ] ¿Envía código o datos a servicios externos? Si es así, se descarta.
- [ ] Licencia compatible y conocida.
- [ ] Estrellas, actividad reciente y mantenedor identificable.
- [ ] Copiar la carpeta a `.claude/skills/<nombre>/` y anotar abajo el commit de origen.

## Recomendadas
| Skill | Origen | Uso | Notas |
|---|---|---|---|
| brainstorming, writing-plans | obra/superpowers (MIT, ~304k★) | Especificar y planificar | Solo estas dos. Telemetría desactivada en `.claude/settings.json` |
| svelte-core-bestpractices, svelte-code-writer | sveltejs/ai-tools (MIT, oficial) | Interfaz Svelte 5 | `svelte-code-writer` ejecuta `npx @sveltejs/mcp` |
| rust-review, insecure-defaults, differential-review | trailofbits/skills (CC-BY-SA-4.0, ~7.6k★) | Seguridad del núcleo Rust | |
| property-based-testing | trailofbits/skills | Pruebas con proptest y fast-check | |
| open-sourcing | trailofbits/skills | Preparar la publicación | Antes de la 0.1 |
| skill-creator | anthropics/skills (Apache-2.0) | Crear skills propias (EXIF, SQLite, miniaturas) | |

## Con cautela
| Skill | Motivo |
|---|---|
| dchuk/claude-code-tauri-skills | Útil para Tauri v2, pero 31★, un commit y sin licencia visible. Leer entera |
| github/spec-kit | Alternativa más formal a Superpowers. Sin extensiones de la comunidad |
| affaan-m/ECC | Solo copiar piezas sueltas (planner, rust-reviewer, strategic-compact). Nunca la instalación completa |

## Descartadas
claude-code-router (envía prompts a terceros), rtk (filtra la salida de comandos), actionbook/rust-skills (hooks invasivos y descargas), udapy/rust-agentic-skills (scripts de shell).

## Instaladas
| Skill | Commit de origen | Fecha | Revisada por |
|---|---|---|---|
| (ninguna todavía) | | | |
