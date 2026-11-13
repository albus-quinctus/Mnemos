<script lang="ts">
  // Sustituye a la barra superior mientras hay selección (spec 006).
  import Icono from "$lib/componentes/Icono.svelte";
  import MenuAnadirAlbum from "$lib/albumes/MenuAnadirAlbum.svelte";
  import { limpiarSeleccion, seleccion } from "./seleccion.svelte";
  import {
    albumAbierto,
    alternarArchivo,
    alternarFavoritos,
    avisos,
    enArchivo,
    enPapelera,
    enviarSeleccionAPapelera,
    quitarSeleccionDeAlbum,
    restaurarSeleccion,
    todasFavoritas,
  } from "./acciones.svelte";
  import { borrado, pedirEliminar } from "$lib/papelera/papelera.svelte";
  import type { TamanoExportacion } from "$lib/biblioteca";
  import { enCurso, exportarVarias } from "$lib/exportacion/exportacion.svelte";

  const cuantas = $derived(seleccion.ids.size);
  const quitar = $derived(todasFavoritas());
  const desarchivar = $derived(enArchivo());
  const papelera = $derived(enPapelera());
  const enAlbum = $derived(albumAbierto() !== null);

  // Exportar (spec 017): el mismo menú que el visor.
  let menuExportar = $state(false);
  let menu = $state<HTMLDivElement>();

  function exportarComo(tamano: TamanoExportacion) {
    menuExportar = false;
    void exportarVarias([...seleccion.ids], tamano);
  }

  function alPulsarFuera(e: PointerEvent) {
    if (menuExportar && menu && !menu.contains(e.target as Node)) menuExportar = false;
  }

  function alPulsarTecla(e: KeyboardEvent) {
    if (e.key === "Escape" && menuExportar) {
      menuExportar = false;
      e.stopPropagation();
    }
  }
</script>

<svelte:window onpointerdown={alPulsarFuera} onkeydowncapture={alPulsarTecla} />

<header class:en-album={enAlbum}>
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
  {#if papelera}
    <!-- En la Papelera (spec 015): restaurar o borrar del disco. -->
    <button
      type="button"
      class="boton"
      disabled={avisos.ocupado || borrado.enCurso}
      onclick={restaurarSeleccion}
    >
      <Icono nombre="restaurar" />
      <span>Restaurar</span>
    </button>
    <button
      type="button"
      class="boton peligro"
      disabled={avisos.ocupado || borrado.enCurso}
      onclick={() => pedirEliminar([...seleccion.ids])}
    >
      <Icono nombre="papelera" />
      <span>Eliminar para siempre</span>
    </button>
  {:else}
    <!-- Álbumes (spec 014). -->
    <MenuAnadirAlbum />
    {#if enAlbum}
      <button
        type="button"
        class="boton"
        title="Quitar del álbum"
        disabled={avisos.ocupado}
        onclick={quitarSeleccionDeAlbum}
      >
        <Icono nombre="album-menos" />
        <span>Quitar del álbum</span>
      </button>
    {/if}
    <button
      type="button"
      class="boton"
      title={quitar ? "Quitar de Favoritos" : "Añadir a Favoritos"}
      disabled={avisos.ocupado}
      onclick={alternarFavoritos}
    >
      <Icono nombre="estrella" relleno={quitar} />
      <span>{quitar ? "Quitar de Favoritos" : "Añadir a Favoritos"}</span>
    </button>
    <button
      type="button"
      class="boton"
      title={desarchivar ? "Desarchivar" : "Archivar"}
      disabled={avisos.ocupado}
      onclick={alternarArchivo}
    >
      <Icono nombre={desarchivar ? "desarchivar" : "archivo"} />
      <span>{desarchivar ? "Desarchivar" : "Archivar"}</span>
    </button>
    <button
      type="button"
      class="boton"
      title="Enviar a la papelera (Supr)"
      disabled={avisos.ocupado}
      onclick={enviarSeleccionAPapelera}
    >
      <Icono nombre="papelera" />
      <span>Papelera</span>
    </button>
  {/if}
  <div class="exportar" bind:this={menu}>
    <button
      type="button"
      class="boton"
      class:activo={menuExportar}
      aria-haspopup="menu"
      aria-expanded={menuExportar}
      title="Exportar"
      disabled={enCurso()}
      onclick={() => (menuExportar = !menuExportar)}
    >
      <Icono nombre="exportar" />
      <span>Exportar</span>
    </button>
    {#if menuExportar}
      <div class="menu" role="menu" aria-label="Exportar">
        <button type="button" role="menuitem" onclick={() => exportarComo("original")}>
          Tamaño original
          <span>Los archivos tal cual, con sus datos</span>
        </button>
        <button type="button" role="menuitem" onclick={() => exportarComo("reducida")}>
          Tamaño reducido (2048 px)
          <span>JPEG para compartir, sin ubicación</span>
        </button>
      </div>
    {/if}
  </div>
</header>

<style>
  header {
    /* Con muchos botones y poco ancho, los botones se quedan en icono. */
    container-type: inline-size;
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
    white-space: nowrap;
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
    white-space: nowrap;
    cursor: pointer;
  }

  /* El texto se oculta a la vista, no a los lectores de pantalla; el
     `title` del botón lo enseña al pasar el ratón. */
  @container (max-width: 1000px) {
    header :global(.boton > span) {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip-path: inset(50%);
    }

    header :global(.boton) {
      width: var(--objetivo);
      padding: 0;
    }
  }

  /* Dentro de un álbum hay un botón más. */
  @container (max-width: 1180px) {
    header.en-album :global(.boton > span) {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip-path: inset(50%);
    }

    header.en-album :global(.boton) {
      width: var(--objetivo);
      padding: 0;
    }
  }

  .boton.solo {
    width: var(--objetivo);
    padding: 0;
  }

  .boton:hover:not(:disabled) {
    background: var(--superficie-alta);
  }

  .boton.peligro {
    color: var(--error);
  }

  .boton.activo {
    background: var(--superficie-alta);
  }

  .exportar {
    position: relative;
  }

  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 5;
    display: grid;
    min-width: 270px;
    padding: 6px;
    border: 1px solid var(--borde);
    border-radius: var(--radio);
    background: var(--superficie);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.18);
  }

  .menu button {
    display: grid;
    gap: 2px;
    padding: 8px 12px;
    border: 0;
    border-radius: calc(var(--radio) - 2px);
    background: transparent;
    color: var(--texto);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .menu button:hover,
  .menu button:focus-visible {
    background: var(--superficie-alta);
    outline: none;
  }

  .menu span {
    color: var(--texto-suave);
    font-size: 0.8rem;
  }

  .boton:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
