<script lang="ts">
  // Título de un día en su cabecera (spec 006): «Añadir título» al pasar el
  // ratón, se escribe en el sitio; Intro o salir del campo guarda, Esc
  // cancela y vacío lo borra. Svelte escapa el texto: nunca se lee como HTML.
  import { avisarFallo } from "$lib/seleccion/seleccion.svelte";
  import { guardarTitulo, MAX_TITULO, titulos } from "./titulos.svelte";

  let { dia }: { dia: string } = $props();

  const titulo = $derived(titulos.get(dia) ?? "");
  let editando = $state(false);
  let texto = $state("");
  let campo = $state<HTMLInputElement>();

  function empezar() {
    texto = titulo;
    editando = true;
  }

  $effect(() => {
    if (editando) campo?.focus();
  });

  async function guardar() {
    if (!editando) return;
    editando = false;
    if (texto.trim() === titulo) return;
    try {
      await guardarTitulo(dia, texto);
    } catch (e) {
      avisarFallo(e);
    }
  }

  function alPulsar(e: KeyboardEvent) {
    // Las teclas del campo no llegan a la cuadrícula (Esc saldría de la
    // búsqueda, Ctrl + A seleccionaría fotos).
    e.stopPropagation();
    if (e.key === "Enter") {
      e.preventDefault();
      guardar();
    } else if (e.key === "Escape") {
      e.preventDefault();
      editando = false;
    }
  }
</script>

{#if editando}
  <input
    bind:this={campo}
    bind:value={texto}
    class="campo"
    type="text"
    maxlength={MAX_TITULO}
    placeholder="Añadir título"
    aria-label="Título del día"
    onkeydown={alPulsar}
    onblur={guardar}
  />
{:else if titulo}
  <span class="separador" aria-hidden="true">·</span>
  <button type="button" class="titulo" title={titulo} onclick={empezar}>{titulo}</button>
{:else}
  <button type="button" class="anadir" onclick={empezar}>Añadir título</button>
{/if}

<style>
  .separador {
    margin: 0 0.4rem;
    color: var(--texto-suave);
  }

  .titulo {
    min-width: 0;
    overflow: hidden;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--texto-suave);
    font: inherit;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: text;
  }

  .titulo:hover {
    color: var(--texto);
  }

  /* Solo se ve al pasar el ratón por la cabecera del día o al enfocarlo. */
  .anadir {
    margin-left: 0.6rem;
    padding: 0.1rem 0.5rem;
    border: 0;
    border-radius: 6px;
    background: var(--acento-suave);
    color: var(--acento);
    font: inherit;
    font-size: 0.85rem;
    opacity: 0;
    cursor: pointer;
  }

  :global(.dia:hover) .anadir,
  .anadir:focus-visible {
    opacity: 1;
  }

  .campo {
    min-width: 0;
    width: 15rem;
    max-width: 100%;
    margin-left: 0.5rem;
    padding: 0.15rem 0.5rem;
    border: 2px solid var(--acento);
    border-radius: 6px;
    outline: none;
    background: var(--superficie);
    color: var(--texto);
    font: inherit;
  }
</style>
