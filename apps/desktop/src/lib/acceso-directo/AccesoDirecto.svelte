<script lang="ts">
  // Entrada al pie del menú lateral: añade o quita MNEMOS del menú del equipo
  // (spec 012). No se muestra si el equipo no lo admite.
  import { onMount } from "svelte";
  import Icono from "../componentes/Icono.svelte";
  import { acceso, anadir, cargarAcceso, quitar } from "./acceso.svelte";
  import { pista } from "./textos";

  onMount(cargarAcceso);
</script>

{#if acceso.disponible}
  <hr />
  <div class="acceso">
    {#if acceso.estado === "si"}
      <div class="fila">
        <span class="estado"><Icono nombre="marca" /><span>En este equipo</span></span>
        <button type="button" class="quitar" disabled={acceso.ocupado} onclick={quitar}>
          Quitar
        </button>
      </div>
    {:else}
      <button
        type="button"
        disabled={acceso.estado === "ajeno" || acceso.ocupado}
        onclick={anadir}
      >
        <Icono nombre="acceso" />
        <span>Añadir a este equipo</span>
      </button>
    {/if}
    <p class="pista" class:error={acceso.estado === "ajeno"}>
      {pista(acceso.sistema, acceso.estado)}
    </p>
  </div>
{/if}

<style>
  hr {
    width: calc(100% - 1.5rem);
    margin: 0.25rem auto;
    border: 0;
    border-top: 1px solid var(--borde);
  }

  .acceso {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  button {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    min-height: var(--objetivo);
    padding: 0 0.75rem;
    border: 0;
    border-radius: var(--radio);
    background: transparent;
    color: var(--texto);
    text-align: left;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    background: var(--superficie-alta);
  }

  button:disabled {
    color: var(--texto-suave);
    cursor: default;
  }

  .fila {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: var(--objetivo);
    padding-left: 0.75rem;
  }

  .estado {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    white-space: nowrap;
  }

  .estado :global(svg) {
    color: var(--acento);
  }

  .quitar {
    flex: none;
    width: auto;
    padding: 0 0.5rem;
    color: var(--acento);
    font-weight: 600;
  }

  .pista {
    margin: 0;
    padding: 0 0.75rem 0.25rem 2.75rem;
    color: var(--texto-suave);
    font-size: 0.8rem;
    line-height: 1.35;
  }

  .pista.error {
    color: var(--error);
  }
</style>
