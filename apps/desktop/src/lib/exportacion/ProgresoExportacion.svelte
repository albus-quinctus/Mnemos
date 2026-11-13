<script lang="ts">
  // Progreso y resumen de una exportación de varias fotos (spec 017).
  import { cancelarExportacion, cerrarExportacion, exportacion } from "./exportacion.svelte";

  const evento = $derived(exportacion.actual);
  let verSaltadas = $state(false);

  const n = (x: number) => x.toLocaleString("es");
  const titulo = $derived.by(() => {
    if (!evento) return "";
    const { estado, total, guardadas, hechas } = evento;
    if (estado === "en_curso") return `Exportando ${n(Math.min(hechas + 1, total))} de ${n(total)}…`;
    if (estado === "cancelada") return "Exportación cancelada";
    if (estado === "detenida") return "Se detuvo la exportación";
    if (guardadas === total) return total === 1 ? "Foto guardada" : `${n(total)} fotos guardadas`;
    return `${n(guardadas)} de ${n(total)} guardadas`;
  });

  $effect(() => {
    if (evento?.estado === "en_curso") verSaltadas = false;
  });
</script>

{#if evento}
  <section class="progreso" aria-live="polite">
    <h2>{titulo}</h2>

    {#if evento.estado === "en_curso"}
      <progress max={evento.total} value={evento.hechas}></progress>
      <p class="cuentas">En {evento.carpeta}</p>
    {:else}
      {#if evento.estado === "detenida" && evento.mensaje}
        <p class="error">{evento.mensaje}</p>
      {/if}
      {#if evento.estado === "terminada"}
        <p class="cuentas">En {evento.ruta ?? evento.carpeta}</p>
      {:else}
        <p class="cuentas">{n(evento.guardadas)} de {n(evento.total)} guardadas en {evento.carpeta}</p>
      {/if}
      {#if evento.saltadas.length > 0}
        <p class="cuentas">
          {evento.saltadas.length === 1
            ? "1 foto no se pudo exportar"
            : `${n(evento.saltadas.length)} fotos no se pudieron exportar`}
        </p>
        {#if verSaltadas}
          <ul class="saltadas">
            {#each evento.saltadas as s (s.nombre + s.motivo)}
              <li><strong>{s.nombre}</strong><span>{s.motivo}</span></li>
            {/each}
          </ul>
        {/if}
      {/if}
    {/if}

    <div class="acciones">
      {#if evento.estado === "en_curso"}
        <button type="button" class="secundario" onclick={cancelarExportacion}>Cancelar</button>
      {:else}
        {#if evento.saltadas.length > 0}
          <button type="button" class="secundario" aria-expanded={verSaltadas} onclick={() => (verSaltadas = !verSaltadas)}>
            {verSaltadas ? "Ocultar" : "Ver"}
          </button>
        {/if}
        <button type="button" onclick={cerrarExportacion}>Cerrar</button>
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
    width: min(24rem, calc(100vw - 2.5rem));
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
    overflow-wrap: anywhere;
    color: var(--texto-suave);
    font-size: 0.9rem;
  }

  .error {
    margin: 0;
    color: var(--error);
    font-size: 0.9rem;
  }

  .saltadas {
    display: grid;
    gap: 0.5rem;
    max-height: 12rem;
    margin: 0.75rem 0 0;
    padding: 0.6rem 0.75rem;
    overflow: auto;
    border-radius: calc(var(--radio) - 2px);
    background: var(--superficie-alta);
    list-style: none;
    font-size: 0.85rem;
  }

  .saltadas li {
    display: grid;
    gap: 1px;
  }

  .saltadas strong {
    overflow-wrap: anywhere;
    font-weight: 600;
  }

  .saltadas span {
    color: var(--texto-suave);
  }

  .acciones {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
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
