<script lang="ts">
  // Sustituye a la barra superior mientras hay selección (spec 006).
  import Icono from "$lib/componentes/Icono.svelte";
  import { limpiarSeleccion, seleccion } from "./seleccion.svelte";
  import {
    alternarArchivo,
    alternarFavoritos,
    avisos,
    enArchivo,
    todasFavoritas,
  } from "./acciones.svelte";

  const cuantas = $derived(seleccion.ids.size);
  const quitar = $derived(todasFavoritas());
  const desarchivar = $derived(enArchivo());
</script>

<header>
  <button
    type="button"
    class="boton solo"
    aria-label="Quitar la selección"
    title="Quitar la selección (Esc)"
    onclick={limpiarSeleccion}
  >
    <Icono nombre="cerrar" />
  </button>
  <span class="cuenta" aria-live="polite">
    {cuantas.toLocaleString("es")}
    {cuantas === 1 ? "seleccionada" : "seleccionadas"}
  </span>
  <button
    type="button"
    class="boton"
    disabled={avisos.ocupado}
    onclick={alternarFavoritos}
  >
    <Icono nombre="estrella" relleno={quitar} />
    <span>{quitar ? "Quitar de Favoritos" : "Añadir a Favoritos"}</span>
  </button>
  <button
    type="button"
    class="boton"
    disabled={avisos.ocupado}
    onclick={alternarArchivo}
  >
    <Icono nombre={desarchivar ? "desarchivar" : "archivo"} />
    <span>{desarchivar ? "Desarchivar" : "Archivar"}</span>
  </button>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    height: var(--barra-alto);
    padding: 0 1.5rem 0 1rem;
    border-bottom: 1px solid var(--borde);
    background: var(--acento-suave);
    color: var(--texto);
  }

  .cuenta {
    margin-right: auto;
    font-size: 1rem;
    font-weight: 600;
  }

  .boton {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    min-width: var(--objetivo);
    height: var(--objetivo);
    padding: 0 0.9rem;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--texto);
    font-weight: 500;
    cursor: pointer;
  }

  .boton.solo {
    width: var(--objetivo);
    padding: 0;
  }

  .boton:hover:not(:disabled) {
    background: var(--superficie-alta);
  }

  .boton:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
