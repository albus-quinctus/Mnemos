<script lang="ts">
  // Fila de Explorar: tarjetas en horizontal que se desplazan con la rueda,
  // el panel táctil o las flechas de los lados (spec 021).
  import type { Snippet } from "svelte";
  import Icono from "$lib/componentes/Icono.svelte";

  let { titulo, nota, children }: { titulo: string; nota?: string; children: Snippet } = $props();

  let lista = $state<HTMLUListElement>();
  let izquierda = $state(false);
  let derecha = $state(false);

  function medir() {
    if (!lista) return;
    izquierda = lista.scrollLeft > 4;
    derecha = lista.scrollLeft + lista.clientWidth < lista.scrollWidth - 4;
  }

  function mover(sentido: 1 | -1) {
    lista?.scrollBy({ left: sentido * lista.clientWidth * 0.8, behavior: "smooth" });
  }

  $effect(() => {
    if (!lista) return;
    medir();
    const observador = new ResizeObserver(medir);
    observador.observe(lista);
    return () => observador.disconnect();
  });
</script>

<section class="fila" aria-label={titulo}>
  <div class="cabecera">
    <h2>{titulo}</h2>
    {#if nota}<span class="nota">{nota}</span>{/if}
  </div>
  <div class="envoltura">
    <ul bind:this={lista} onscroll={medir}>
      {@render children()}
    </ul>
    {#if izquierda}
      <button type="button" class="flecha izq" aria-label="Anteriores" onclick={() => mover(-1)}>
        <Icono nombre="anterior" />
      </button>
    {/if}
    {#if derecha}
      <div class="degradado" aria-hidden="true"></div>
      <button type="button" class="flecha der" aria-label="Siguientes" onclick={() => mover(1)}>
        <Icono nombre="siguiente" />
      </button>
    {/if}
  </div>
</section>

<style>
  .fila {
    margin-top: 1.9rem;
  }

  .cabecera {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    margin-bottom: 0.8rem;
  }

  h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .nota {
    color: var(--texto-suave);
    font-size: 0.9rem;
  }

  .envoltura {
    position: relative;
  }

  ul {
    display: flex;
    gap: 1.25rem;
    margin: 0;
    padding: 4px 4px 6px;
    margin-left: -4px;
    overflow-x: auto;
    scrollbar-width: none;
    list-style: none;
    scroll-snap-type: x proximity;
  }

  ul::-webkit-scrollbar {
    display: none;
  }

  ul :global(li) {
    flex: none;
    scroll-snap-align: start;
  }

  .flecha {
    position: absolute;
    top: calc(50% - 38px);
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    padding: 0;
    border: 1px solid var(--borde);
    border-radius: 50%;
    background: var(--superficie);
    color: var(--texto);
    box-shadow: 0 2px 8px rgb(0 0 0 / 18%);
    cursor: pointer;
  }

  .flecha:hover {
    background: var(--superficie-alta);
  }

  .izq {
    left: -14px;
  }

  .der {
    right: -14px;
  }

  .degradado {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 80px;
    background: linear-gradient(to right, transparent, var(--fondo));
    pointer-events: none;
  }
</style>
