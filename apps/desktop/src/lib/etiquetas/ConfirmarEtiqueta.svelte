<script lang="ts">
  // Confirmaciones de la spec 016: borrar una etiqueta (sus fotos no se
  // borran) o unirla con otra que ya se llama como el nombre nuevo.
  import { tick } from "svelte";
  import { textoFotos } from "./etiquetas";
  import { cancelarConfirmacion, confirmar, estadoEtiquetas } from "./estado.svelte";

  let cancelar = $state<HTMLButtonElement>();
  const c = $derived(estadoEtiquetas.confirmacion);

  $effect(() => {
    if (c) tick().then(() => cancelar?.focus());
  });

  function alPulsar(e: KeyboardEvent) {
    if (e.key === "Escape" && c) {
      e.preventDefault();
      e.stopPropagation();
      cancelarConfirmacion();
    }
  }
</script>

<svelte:window onkeydowncapture={alPulsar} />

{#if c}
  <div class="velo" role="presentation">
    <div
      class="dialogo"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="confirmar-titulo"
      aria-describedby="confirmar-texto"
    >
      {#if c.tipo === "borrar"}
        <h2 id="confirmar-titulo">¿Borrar la etiqueta «{c.etiqueta.name}»?</h2>
        <p id="confirmar-texto">
          {#if c.etiqueta.photos > 0}
            Se quitará de {textoFotos(c.etiqueta.photos)}. Las fotos no se borran.
          {:else}
            No tiene fotos. Las fotos no se borran nunca.
          {/if}
        </p>
      {:else}
        <h2 id="confirmar-titulo">Ya existe «{c.en.name}». ¿Unir las dos?</h2>
        <p id="confirmar-texto">
          Las fotos de «{c.desde.name}» pasarán a «{c.en.name}» y «{c.desde.name}»
          desaparecerá.
        </p>
      {/if}
      <div class="acciones">
        <button type="button" class="cancelar" bind:this={cancelar} onclick={cancelarConfirmacion}>
          Cancelar
        </button>
        <button type="button" class={c.tipo === "borrar" ? "peligro" : "principal"} onclick={confirmar}>
          {c.tipo === "borrar" ? "Borrar etiqueta" : "Unir"}
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

  .peligro {
    background: var(--error);
    color: var(--superficie);
  }

  .principal {
    background: var(--acento);
    color: var(--acento-texto);
  }
</style>
