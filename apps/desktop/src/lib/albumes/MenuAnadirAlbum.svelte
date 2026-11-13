<script lang="ts">
  // «Añadir a álbum» de la barra de selección (spec 014): «Nuevo álbum…»
  // arriba y debajo los álbumes, en el orden de la pantalla Álbumes, con
  // buscador si hay más de 8.
  import { tick } from "svelte";
  import Icono from "$lib/componentes/Icono.svelte";
  import { miniaturaUrl } from "$lib/media";
  import { anadirSeleccionAAlbum, avisos, crearAlbumConSeleccion } from "$lib/seleccion/acciones.svelte";
  import { ALBUMES_SIN_BUSCADOR, filtrarAlbumes, textoFotos } from "./albumes";
  import { cargarAlbumes, estadoAlbumes } from "./estado.svelte";
  import NuevoAlbum from "./NuevoAlbum.svelte";

  let abierto = $state(false);
  let creando = $state(false);
  let texto = $state("");
  let caja = $state<HTMLDivElement>();
  let buscador = $state<HTMLInputElement>();

  const conBuscador = $derived(estadoAlbumes.lista.length > ALBUMES_SIN_BUSCADOR);
  const visibles = $derived(filtrarAlbumes(estadoAlbumes.lista, texto));

  async function alternar() {
    abierto = !abierto;
    if (!abierto) return;
    texto = "";
    await cargarAlbumes();
    await tick();
    buscador?.focus();
  }

  function nuevo() {
    abierto = false;
    creando = true;
  }

  async function crear(nombre: string) {
    const error = await crearAlbumConSeleccion(nombre);
    if (!error) {
      creando = false;
      cargarAlbumes();
    }
    return error;
  }

  function elegir(id: number, nombre: string) {
    abierto = false;
    void anadirSeleccionAAlbum(id, nombre).then(cargarAlbumes);
  }

  function alPulsarFuera(e: PointerEvent) {
    if (abierto && caja && !caja.contains(e.target as Node)) abierto = false;
  }

  function alPulsarTecla(e: KeyboardEvent) {
    if (e.key === "Escape" && abierto) {
      abierto = false;
      e.stopPropagation();
    }
  }

  // Intro en el buscador elige el primero que queda.
  function alPulsarBuscador(e: KeyboardEvent) {
    if (e.key === "Enter" && visibles.length > 0) {
      e.preventDefault();
      elegir(visibles[0].id, visibles[0].name);
    }
  }
</script>

<svelte:window onpointerdown={alPulsarFuera} onkeydowncapture={alPulsarTecla} />

<div class="anadir" bind:this={caja}>
  <button
    type="button"
    class="boton"
    class:activo={abierto}
    aria-haspopup="menu"
    aria-expanded={abierto}
    title="Añadir a álbum"
    disabled={avisos.ocupado}
    onclick={alternar}
  >
    <Icono nombre="album-mas" />
    <span>Añadir a álbum</span>
  </button>
  {#if abierto}
    <div class="menu" role="menu" aria-label="Añadir a álbum">
      <button type="button" role="menuitem" class="nuevo" onclick={nuevo}>
        <span class="icono"><Icono nombre="mas" /></span>
        Nuevo álbum…
      </button>
      {#if estadoAlbumes.lista.length > 0}
        <hr />
      {/if}
      {#if conBuscador}
        <label class="buscador">
          <Icono nombre="buscar" tamano={16} />
          <input
            bind:this={buscador}
            bind:value={texto}
            placeholder="Buscar álbum"
            aria-label="Buscar álbum"
            onkeydown={alPulsarBuscador}
          />
        </label>
      {/if}
      <div class="lista">
        {#each visibles as album (album.id)}
          <button
            type="button"
            role="menuitem"
            class="album"
            onclick={() => elegir(album.id, album.name)}
          >
            {#if album.cover !== null}
              <img
                src={miniaturaUrl(album.cover, album.cover_edit_version ?? undefined)}
                alt=""
                loading="lazy"
              />
            {:else}
              <span class="portada-vacia"><Icono nombre="albumes" tamano={18} /></span>
            {/if}
            <span class="textos">
              <span class="nombre">{album.name}</span>
              <span class="fotos">{textoFotos(album.photos)}</span>
            </span>
          </button>
        {:else}
          {#if texto}
            <p class="nada">Ningún álbum se llama así</p>
          {/if}
        {/each}
      </div>
    </div>
  {/if}
</div>

{#if creando}
  <NuevoAlbum onCrear={crear} onCancelar={() => (creando = false)} />
{/if}

<style>
  .anadir {
    position: relative;
  }

  .boton {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
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

  .boton:hover:not(:disabled),
  .boton.activo {
    background: var(--superficie-alta);
  }

  .boton:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 5;
    display: flex;
    flex-direction: column;
    width: 20rem;
    max-height: min(32rem, calc(100vh - 6rem));
    padding: 6px;
    border: 1px solid var(--borde);
    border-radius: var(--radio);
    background: var(--superficie);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.18);
  }

  .menu button {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-height: var(--objetivo);
    padding: 6px 8px;
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

  .nuevo {
    color: var(--acento) !important;
    font-weight: 600 !important;
  }

  .icono,
  .portada-vacia {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 6px;
    background: var(--acento-suave);
  }

  .portada-vacia {
    background: var(--superficie-alta);
    color: var(--texto-suave);
  }

  hr {
    width: 100%;
    margin: 4px 0;
    border: 0;
    border-top: 1px solid var(--borde);
  }

  .buscador {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 2px 0 4px;
    padding: 0 0.75rem;
    border-radius: calc(var(--radio) - 2px);
    background: var(--superficie-alta);
    color: var(--texto-suave);
  }

  .buscador input {
    flex: 1;
    min-width: 0;
    height: 36px;
    border: 0;
    background: transparent;
    color: var(--texto);
    font: inherit;
    outline: none;
  }

  .lista {
    overflow-y: auto;
  }

  .album img {
    flex: none;
    width: 36px;
    height: 36px;
    border-radius: 6px;
    object-fit: cover;
  }

  .textos {
    display: grid;
    min-width: 0;
  }

  .nombre {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .fotos {
    color: var(--texto-suave);
    font-size: 0.85rem;
  }

  .nada {
    margin: 0.5rem 0.75rem;
    color: var(--texto-suave);
  }
</style>
