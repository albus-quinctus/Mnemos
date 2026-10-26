<script lang="ts">
  import {
    cancelarImportacion,
    cerrarImportacion,
    importacion,
  } from "./importacion.svelte";

  const evento = $derived(importacion.actual);
  const enCurso = $derived(evento?.estado === "en_curso");
  const titulo = $derived(
    {
      en_curso: "Importando fotos",
      terminada: "Importación terminada",
      cancelada: "Importación cancelada",
      fallida: "La importación falló",
    }[evento?.estado ?? "terminada"],
  );
</script>

{#if evento}
  <section class="progreso" aria-live="polite">
    <h2>{titulo}</h2>

    {#if evento.estado === "fallida"}
      <p class="error">{evento.mensaje}</p>
    {:else if evento.total > 0}
      <progress max={evento.total} value={evento.procesadas}></progress>
      <p class="cuentas">{evento.procesadas} de {evento.total}</p>
    {:else if enCurso}
      <p class="cuentas">Buscando fotos…</p>
    {/if}

    {#if evento.estado !== "fallida"}
      <p class="cuentas">
        {evento.copiadas} copiadas · {evento.duplicadas} repetidas · {evento.errores} con error
      </p>
    {/if}

    <div class="acciones">
      {#if enCurso}
        <button type="button" class="secundario" onclick={cancelarImportacion}>Cancelar</button>
      {:else}
        <button type="button" onclick={cerrarImportacion}>Cerrar</button>
      {/if}
    </div>
  </section>
{/if}

<style>
  .progreso {
    position: fixed;
    right: 1.25rem;
    bottom: 1.25rem;
    z-index: 20;
    width: min(22rem, calc(100vw - 2.5rem));
    padding: 1rem 1.1rem;
    border: 1px solid var(--borde);
    border-radius: var(--radio);
    background: var(--superficie);
    color: var(--texto);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.18);
  }

  h2 {
    margin: 0 0 0.6rem;
    font-size: 1rem;
    font-weight: 600;
  }

  progress {
    width: 100%;
    height: 0.5rem;
  }

  .cuentas {
    margin: 0.5rem 0 0;
    color: var(--texto-suave);
    font-size: 0.9rem;
  }

  .error {
    margin: 0;
    color: var(--error);
    font-size: 0.9rem;
  }

  .acciones {
    display: flex;
    justify-content: flex-end;
    margin-top: 0.9rem;
  }

  button {
    min-height: var(--objetivo);
    padding: 0 1.1rem;
    border: 0;
    border-radius: var(--radio);
    background: var(--acento);
    color: var(--acento-texto);
    font-weight: 600;
    cursor: pointer;
  }

  button.secundario {
    border: 1px solid var(--borde);
    background: var(--superficie-alta);
    color: var(--texto);
  }
</style>
