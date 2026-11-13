<script lang="ts">
  // Diálogo antes de borrar del disco (spec 015). El foco empieza en
  // «Cancelar», así que Intro no borra: hay que pulsar el botón rojo.
  import { tick } from "svelte";
  import { aceptarConfirmacion, cancelarConfirmacion, confirmacion } from "./papelera.svelte";

  let cancelar = $state<HTMLButtonElement>();

  $effect(() => {
    if (confirmacion.actual) tick().then(() => cancelar?.focus());
  });

  function alPulsar(e: KeyboardEvent) {
    if (!confirmacion.actual) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      cancelarConfirmacion();
    }
  }
</script>

<svelte:window onkeydowncapture={alPulsar} />

{#if confirmacion.actual}
  <div class="velo" role="presentation">
    <div
      class="dialogo"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="confirmar-titulo"
      aria-describedby="confirmar-texto"
    >
      <h2 id="confirmar-titulo">{confirmacion.actual.titulo}</h2>
      <p id="confirmar-texto">{confirmacion.actual.texto}</p>
      <div class="acciones">
        <button type="button" class="cancelar" bind:this={cancelar} onclick={cancelarConfirmacion}>
          Cancelar
        </button>
        <button type="button" class="peligro" onclick={aceptarConfirmacion}>
          {confirmacion.actual.boton}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .velo {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    background: rgb(0 0 0 / 0.45);
  }

  .dialogo {
    width: min(26rem, 100%);
    padding: 1.5rem;
    border-radius: 14px;
    background: var(--superficie);
    color: var(--texto);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.35);
  }

  h2 {
    margin: 0 0 0.5rem;
    font-size: 1.15rem;
    font-weight: 600;
  }

  p {
    margin: 0 0 1.5rem;
    color: var(--texto-suave);
  }

  .acciones {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  button {
    min-height: var(--objetivo);
    padding: 0 1.1rem;
    border: 0;
    border-radius: 999px;
    font-weight: 600;
    cursor: pointer;
  }

  .cancelar {
    border: 1px solid var(--borde);
    background: transparent;
    color: var(--texto);
  }

  .cancelar:focus {
    outline: 2px solid var(--acento);
    outline-offset: 2px;
  }

  .peligro {
    background: var(--error);
    color: var(--superficie);
  }
</style>
