<script lang="ts">
  // Diálogo «Nuevo álbum» (spec 014): solo pide el nombre. Intro crea y Esc
  // cancela. Se usa desde la pantalla Álbumes y desde la barra de selección.
  import { tick } from "svelte";
  import { MAX_NOMBRE, nombreValido } from "./albumes";

  let {
    onCrear,
    onCancelar,
  }: {
    /** Crea el álbum; devuelve el error para enseñarlo, o null. */
    onCrear: (nombre: string) => Promise<string | null>;
    onCancelar: () => void;
  } = $props();

  let nombre = $state("");
  let error = $state("");
  let ocupado = $state(false);
  let campo = $state<HTMLInputElement>();
  const valido = $derived(nombreValido(nombre));

  $effect(() => {
    tick().then(() => campo?.focus());
  });

  async function crear(e: SubmitEvent) {
    e.preventDefault();
    if (!valido || ocupado) return;
    ocupado = true;
    error = (await onCrear(valido)) ?? "";
    ocupado = false;
  }

  function alPulsar(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onCancelar();
    }
  }
</script>

<svelte:window onkeydowncapture={alPulsar} />

<div class="velo" role="presentation">
  <div class="dialogo" role="dialog" aria-modal="true" aria-labelledby="nuevo-titulo">
    <form onsubmit={crear}>
      <h2 id="nuevo-titulo">Nuevo álbum</h2>
      <input
        bind:this={campo}
        bind:value={nombre}
        maxlength={MAX_NOMBRE}
        placeholder="Nombre del álbum"
        aria-label="Nombre del álbum"
      />
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
      <div class="acciones">
        <button type="button" class="cancelar" onclick={onCancelar}>Cancelar</button>
        <button type="submit" class="crear" disabled={!valido || ocupado}>Crear</button>
      </div>
    </form>
  </div>
</div>

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
    margin: 0 0 0.75rem;
    font-size: 1.15rem;
    font-weight: 600;
  }

  input {
    width: 100%;
    height: 44px;
    padding: 0 0.8rem;
    border: 2px solid var(--borde);
    border-radius: var(--radio);
    background: var(--superficie);
  }

  input:focus {
    border-color: var(--acento);
    outline: none;
  }

  .error {
    margin: 0.5rem 0 0;
    color: var(--error);
    font-size: 0.9rem;
  }

  .acciones {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1.4rem;
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

  .crear {
    background: var(--acento);
    color: var(--acento-texto);
  }

  .crear:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
