<script lang="ts">
  // Aviso abajo durante 8 s tras una acción, con «Deshacer» (spec 006) o la
  // acción que se indique, como «Quitar» en el acceso directo (spec 012).
  import { avisos, cerrarAviso, deshacer } from "./acciones.svelte";
</script>

{#if avisos.actual}
  <div class="aviso" class:error={avisos.actual.error} role="status">
    <span>{avisos.actual.texto}</span>
    {#if avisos.actual.deshacer}
      <button type="button" onclick={deshacer}>{avisos.actual.etiqueta ?? "Deshacer"}</button>
    {:else}
      <button type="button" onclick={cerrarAviso}>Cerrar</button>
    {/if}
  </div>
{/if}

<style>
  .aviso {
    position: fixed;
    bottom: 28px;
    left: 50%;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 1.25rem;
    max-width: min(40rem, calc(100vw - 2rem));
    padding: 0.75rem 0.75rem 0.75rem 1.25rem;
    border-radius: 10px;
    background: #2b2c30;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.3);
    color: #f1f1f3;
    font-size: 0.95rem;
    transform: translateX(-50%);
  }

  .aviso.error span {
    color: #f2938c;
  }

  button {
    flex: none;
    min-height: var(--objetivo);
    padding: 0.4rem 0.6rem;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: #9cc0ff;
    font-weight: 600;
    cursor: pointer;
  }

  button:hover {
    background: rgba(255, 255, 255, 0.08);
  }
</style>
