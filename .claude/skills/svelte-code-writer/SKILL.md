---
name: svelte-code-writer
description: CLI tools for Svelte 5 documentation lookup and code analysis. MUST be used whenever creating, editing or analyzing any Svelte component (`.svelte`) or Svelte module (`.svelte.ts`/`.svelte.js`). If possible, this skill should be executed within the `svelte-file-editor` subagent for optimal results.
---

> Copiada de [sveltejs/ai-tools](https://github.com/sveltejs/ai-tools) (commit
> `b2589f89`, MIT). Único cambio: la versión de `@sveltejs/mcp` está fijada
> para no ejecutar código nuevo sin revisarlo. `get-documentation` y
> `list-sections` descargan la documentación pública de svelte.dev; el
> autofixer funciona en local. Ver `docs/skills.md`.

## CLI tools

You have access to `@sveltejs/mcp@0.1.27` CLI for Svelte-specific assistance. Use these commands via `npx`:

### List documentation sections

```bash
npx @sveltejs/mcp@0.1.27 list-sections
```

Lists all available Svelte 5 and SvelteKit documentation sections with titles and paths.

### Get documentation

```bash
npx @sveltejs/mcp@0.1.27 get-documentation "<section1>,<section2>,..."
```

Retrieves full documentation for specified sections. Use after `list-sections` to fetch relevant docs.

**Example:**

```bash
npx @sveltejs/mcp@0.1.27 get-documentation "$state,$derived,$effect"
```

### Svelte autofixer

```bash
npx @sveltejs/mcp@0.1.27 svelte-autofixer "<code_or_path>" [options]
```

Analyzes Svelte code and suggests fixes for common issues.

**Options:**

- `--async` - Enable async Svelte mode (default: false)
- `--svelte-version` - Target version, ideally the full version from `package.json` (e.g. `5.16.0`) so version-specific suggestions are enabled. The major must be 4 or 5 (default: 5)

**Examples:**

```bash
# Analyze inline code (escape $ as \$)
npx @sveltejs/mcp@0.1.27 svelte-autofixer '<script>let count = \$state(0);</script>'

# Analyze a file
npx @sveltejs/mcp@0.1.27 svelte-autofixer ./src/lib/Component.svelte

# Target Svelte 4
npx @sveltejs/mcp@0.1.27 svelte-autofixer ./Component.svelte --svelte-version 4
```

**Important:** When passing code with runes (`$state`, `$derived`, etc.) via the terminal, escape the `$` character as `\$` to prevent shell variable substitution.

## Workflow

1. **Uncertain about syntax?** Run `list-sections` then `get-documentation` for relevant topics
2. **Reviewing/debugging?** Run `svelte-autofixer` on the code to detect issues
3. **Always validate** - Run `svelte-autofixer` before finalizing any Svelte component
