<script lang="ts">
  // Cabecera de un álbum abierto (spec 014): volver a Álbumes, nombre
  // editable en el sitio, número de fotos y fechas, y el menú ⋮ con «Cambiar
  // nombre» y «Eliminar álbum». Eliminar nunca borra fotos.
  import { tick } from "svelte";
  import { album as pedirAlbum, eliminarAlbum, renombrarAlbum, type Album } from "$lib/biblioteca";
  import Icono from "$lib/componentes/Icono.svelte";
  import { irA } from "$lib/navegacion.svelte";
  import { MAX_NOMBRE, nombreValido, subtituloAlbum } from "./albumes";
  import { cargarAlbumes } from "./estado.svelte";

  let { id, version = 0 }: { id: number; version?: number } = $props();

  let album = $state<Album | null>(null);
  let error = $state("");
  let editando = $state(false);
  let borrador = $state("");
  let menuAbierto = $state(false);
  let confirmando = $state(false);
  let campo = $state<HTMLInputElement>();
  let cancelar = $state<HTMLButtonElement>();

  // Al abrir y cada vez que cambian sus fotos (`version`).
  $effect(() => {
    void version;
    pedirAlbum(id)
      .then((a) => (album = a))
      .catch((e) => (error = String(e)));
  });

  async function empezarAEditar() {
    if (!album) return;
    menuAbierto = false;
    borrador = album.name;
    editando = true;
    await tick();
    campo?.select();
  }

  async function guardar() {
    if (!editando || !album) return;
    editando = false;
    const nombre = nombreValido(borrador);
    if (!nombre || nombre === album.name) return;
    try {
      album.name = await renombrarAlbum(id, nombre);
      cargarAlbumes();
    } catch (e) {
      error = String(e);
    }
  }

  function alPulsarCampo(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      guardar();
    } else if (e.key === "Escape") {
      // Que no llegue a la cuadrícula: Esc solo cancela la edición.
      e.preventDefault();
      e.stopPropagation();
      editando = false;
    }
  }

  async function pedirEliminar() {
    menuAbierto = false;
    confirmando = true;
    await tick();
    cancelar?.focus();
  }

  async function eliminar() {
    try {
      await eliminarAlbum(id);
      confirmando = false;
      irA("albumes");
    } catch (e) {
      confirmando = false;
      error = String(e);
    }
  }

  function alPulsarVentana(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    if (confirmando) confirmando = false;
    else if (menuAbierto) menuAbierto = false;
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<svelte:window onkeydowncapture={alPulsarVentana} />

<header class="cabecera">
  <button type="button" class="volver" onclick={() => irA("albumes")}>
    <Icono nombre="anterior" tamano={18} />
    <span>Álbumes</span>
  </button>
  <div class="fila">
    <div class="textos">
      {#if editando}
        <input
          bind:this={campo}
          bind:value={borrador}
          class="titulo"
          maxlength={MAX_NOMBRE}
          aria-label="Nombre del álbum"
          onkeydown={alPulsarCampo}
          onblur={guardar}
        />
      {:else}
        <h1>
          <button type="button" class="titulo" title="Cambiar nombre" onclick={empezarAEditar}>
            {album?.name ?? ""}
          </button>
        </h1>
      {/if}
      {#if album}
        <p class="subtitulo">{subtituloAlbum(album.photos, album.first, album.last)}</p>
      {/if}
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
    </div>
    <div class="menu">
      <button
        type="button"
        class="puntos"
        aria-label="Más opciones del álbum"
        aria-haspopup="menu"
        aria-expanded={menuAbierto}
        onclick={() => (menuAbierto = !menuAbierto)}
      >
        <Icono nombre="puntos" grosor={3} />
      </button>
      {#if menuAbierto}
        <div class="desplegable" role="menu">
          <button type="button" role="menuitem" onclick={empezarAEditar}>Cambiar nombre</button>
          <button type="button" role="menuitem" class="peligro" onclick={pedirEliminar}>
            Eliminar álbum
          </button>
        </div>
      {/if}
    </div>
  </div>
</header>

{#if confirmando && album}
  <div class="velo" role="presentation">
    <div
      class="dialogo"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="eliminar-titulo"
      aria-describedby="eliminar-texto"
    >
      <h2 id="eliminar-titulo">¿Eliminar el álbum «{album.name}»?</h2>
      <p id="eliminar-texto">Las fotos no se borran: siguen en tu biblioteca.</p>
      <div class="acciones">
        <button type="button" class="cancelar" bind:this={cancelar} onclick={() => (confirmando = false)}>
          Cancelar
        </button>
        <button type="button" class="eliminar" onclick={eliminar}>Eliminar álbum</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .cabecera {
    padding: 1.25rem 4.5rem 0.25rem 2rem;
  }

  .volver {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    height: 32px;
    margin-left: -0.5rem;
    padding: 0 0.6rem 0 0.4rem;
    border: 0;
    border-radius: 999px;
    background: none;
    color: var(--texto-suave);
    font-weight: 500;
    cursor: pointer;
  }

  .volver:hover {
    background: var(--superficie-alta);
  }

  .fila {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    margin-top: 0.4rem;
  }

  .textos {
    flex: 1;
    min-width: 0;
  }

  h1 {
    margin: 0;
  }

  .titulo {
    display: block;
    max-width: 100%;
    margin-left: -0.35rem;
    padding: 0 0.35rem;
    overflow: hidden;
    border: 2px solid transparent;
    border-radius: 6px;
    background: none;
    color: var(--texto);
    font-size: 2rem;
    font-weight: 650;
    line-height: 1.3;
    text-align: left;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  button.titulo {
    cursor: text;
  }

  button.titulo:hover {
    background: var(--superficie-alta);
  }

  input.titulo {
    width: 100%;
    border-color: var(--acento);
    outline: none;
  }

  .subtitulo {
    margin: 0.15rem 0 0;
    color: var(--texto-suave);
  }

  .error {
    margin: 0.25rem 0 0;
    color: var(--error);
  }

  .menu {
    position: relative;
    margin-top: 0.3rem;
  }

  .puntos {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--objetivo);
    height: var(--objetivo);
    border: 0;
    border-radius: 999px;
    background: none;
    color: var(--texto);
    cursor: pointer;
  }

  .puntos:hover,
  .puntos[aria-expanded="true"] {
    background: var(--superficie-alta);
  }

  .desplegable {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 30;
    display: flex;
    flex-direction: column;
    width: 14rem;
    padding: 0.4rem;
    border: 1px solid var(--borde);
    border-radius: 12px;
    background: var(--superficie);
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.18);
  }

  .desplegable button {
    height: 44px;
    padding: 0 0.75rem;
    border: 0;
    border-radius: 8px;
    background: none;
    color: var(--texto);
    text-align: left;
    cursor: pointer;
  }

  .desplegable button:hover,
  .desplegable button:focus-visible {
    background: var(--superficie-alta);
    outline: none;
  }

  .desplegable .peligro {
    color: var(--error);
  }

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

  .dialogo h2 {
    margin: 0 0 0.5rem;
    font-size: 1.15rem;
    font-weight: 600;
  }

  .dialogo p {
    margin: 0 0 1.5rem;
    color: var(--texto-suave);
  }

  .acciones {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .acciones button {
    min-height: var(--objetivo);
    padding: 0 1.1rem;
    border: 0;
    border-radius: 999px;
    font-weight: 600;
    cursor: pointer;
  }

  .cancelar {
    border: 1px solid var(--borde) !important;
    background: transparent;
    color: var(--texto);
  }

  .cancelar:focus {
    outline: 2px solid var(--acento);
    outline-offset: 2px;
  }

  .eliminar {
    background: var(--error);
    color: var(--superficie);
  }
</style>
