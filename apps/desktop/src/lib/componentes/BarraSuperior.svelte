<script lang="ts">
  import { busqueda, escribir, limpiarBusqueda } from "$lib/busqueda/busqueda.svelte";
  import { importacion, importarCarpeta } from "$lib/importacion/importacion.svelte";
  import Icono from "./Icono.svelte";

  let campo = $state<HTMLInputElement>();

  function alPulsar(e: KeyboardEvent) {
    if (e.key === "Escape") {
      limpiarBusqueda();
      campo?.blur();
    }
  }
</script>

<header>
  <label class="buscar">
    <Icono nombre="buscar" />
    <input
      type="search"
      placeholder="Buscar en tus fotos"
      aria-label="Buscar por nombre, carpeta o fecha"
      bind:this={campo}
      value={busqueda.texto}
      oninput={(e) => escribir(e.currentTarget.value)}
      onkeydown={alPulsar}
      disabled={!busqueda.disponible}
    />
    {#if busqueda.texto}
      <button
        type="button"
        class="limpiar"
        aria-label="Borrar la búsqueda"
        onclick={() => {
          limpiarBusqueda();
          campo?.focus();
        }}
      >
        <Icono nombre="cerrar" />
      </button>
    {/if}
  </label>
  <button
    type="button"
    class="importar"
    onclick={importarCarpeta}
    disabled={importacion.actual?.estado === "en_curso"}
  >
    <Icono nombre="importar" />
    <span>Importar</span>
  </button>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 1rem;
    height: var(--barra-alto);
    padding: 0 1.5rem;
    border-bottom: 1px solid var(--borde);
    background: var(--fondo);
  }

  .buscar {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 0.6rem;
    max-width: 40rem;
    min-height: var(--objetivo);
    padding: 0 0.9rem;
    border-radius: var(--radio);
    background: var(--superficie-alta);
    color: var(--texto-suave);
  }

  input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    outline: none;
  }

  .buscar:focus-within {
    outline: 2px solid var(--acento);
    outline-offset: -2px;
  }

  input {
    color: var(--texto);
  }

  input::placeholder {
    color: var(--texto-suave);
  }

  /* La cruz propia sustituye a la del sistema. */
  input::-webkit-search-cancel-button {
    display: none;
  }

  .limpiar {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    margin-right: -0.5rem;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--texto-suave);
    cursor: pointer;
  }

  .limpiar:hover {
    background: var(--superficie);
    color: var(--texto);
  }

  .importar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-height: var(--objetivo);
    margin-left: auto;
    padding: 0 1rem;
    border: 1px solid var(--borde);
    border-radius: var(--radio);
    background: var(--superficie);
    cursor: pointer;
  }

  .importar:disabled {
    color: var(--texto-suave);
    cursor: default;
  }
</style>
