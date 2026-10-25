# Skills de IA del proyecto

Evaluadas el 2026-10-23 por sus README. Revisadas archivo por archivo e instaladas el 2026-10-25. Cada skill nueva pasa por la lista de comprobación antes de copiarse a `.claude/skills/`.

## Lista de comprobación antes de instalar
- [ ] Leer `SKILL.md` completo y todos los archivos de su carpeta.
- [ ] ¿Ejecuta scripts, `npx`, `curl` o descargas? ¿Qué hacen exactamente?
- [ ] ¿Instala hooks o toca `settings.json`? Si es así, se descarta salvo decisión explícita.
- [ ] ¿Envía código o datos a servicios externos? Si es así, se descarta.
- [ ] Licencia compatible y conocida.
- [ ] Estrellas, actividad reciente y mantenedor identificable.
- [ ] Copiar la carpeta a `.claude/skills/<nombre>/` y anotar abajo el commit de origen.

## Instaladas
Revisadas el 2026-10-25 leyendo todos sus archivos y, en `svelte-code-writer`, el código del paquete que ejecuta.

| Skill | Origen y commit | Licencia | Cambios | Qué hace con permisos |
|---|---|---|---|---|
| `brainstorming` | obra/superpowers `2ce8aa15` | MIT | Sin "visual companion" (no arranca el servidor Node local ni su logo remoto); specs en `docs/specs/` | Solo texto. Para proyectos grandes lanza un subagente que revisa la spec |
| `writing-plans` | obra/superpowers `2ce8aa15` | MIT | Planes en `docs/planes/`; sin worktrees ni subagentes de ejecución; commits en Conventional Commits | Solo texto |
| `svelte-core-bestpractices` | sveltejs/ai-tools `b2589f89` | MIT | Ninguno | Solo texto |
| `svelte-code-writer` | sveltejs/ai-tools `b2589f89` | MIT | Versión fijada: `@sveltejs/mcp@0.1.27` | `npx` (pide permiso). Descarga la documentación pública de svelte.dev; el autofixer es local |
| `property-based-testing` | trailofbits/skills `196a9073` | CC-BY-SA-4.0 | Ninguno (sin evals ni iconos) | Solo texto y búsquedas con `rg` |
| `differential-review` | trailofbits/skills `196a9073` | CC-BY-SA-4.0 | Ninguno | Comandos `git` de lectura. Su metodología propone `git checkout` del commit base: hacerlo solo sin cambios pendientes |
| `open-sourcing` | trailofbits/skills `196a9073` | CC-BY-SA-4.0 | Ninguno | Dos scripts de solo lectura (`check_readiness.sh`, `detect_org.sh`) |

Cada carpeta conserva la licencia original en su `LICENSE`. Las skills son documentación para el asistente, no forman parte de la aplicación, así que sus licencias no afectan a la GPL de MNEMOS.

## Pospuestas
| Skill | Motivo |
|---|---|
| trailofbits `rust-review` | 420 KB, orquesta muchos subagentes y scripts Python: caro en contexto. Pensada para `unsafe` y FFI, que MNEMOS prohíbe. Instalar para la revisión de seguridad antes de la 0.5 |
| trailofbits `insecure-defaults` | No es una skill sino una orden con un flujo en JavaScript. Revisar antes de la 0.5 |
| anthropics `skill-creator` | Ya viene incluida en Claude; la copia del repositorio añade scripts Python que no necesitamos |
| dchuk/claude-code-tauri-skills | 31★, un commit y sin licencia visible: sin licencia no se puede copiar |

## Con cautela
| Skill | Motivo |
|---|---|
| github/spec-kit | Alternativa más formal a Superpowers. Sin extensiones de la comunidad |
| affaan-m/ECC | Solo copiar piezas sueltas (planner, rust-reviewer, strategic-compact). Nunca la instalación completa |

## Descartadas
claude-code-router (envía prompts a terceros), rtk (filtra la salida de comandos), actionbook/rust-skills (hooks invasivos y descargas), udapy/rust-agentic-skills (scripts de shell).

## Actualizar una skill
1. Leer el diff entre el commit anotado y el nuevo.
2. Pasar la lista de comprobación de arriba.
3. Volver a aplicar los cambios de la columna "Cambios" y actualizar el commit en esta tabla.
