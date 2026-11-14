<script lang="ts">
  // Cabecera de las fotos de una etiqueta (spec 016): volver a Etiquetas,
  // nombre editable en el sitio, número de fotos y el menú ⋮ con «Renombrar»
  // y «Borrar». Borrar nunca borra fotos.
  import { tick } from "svelte";
  import Icono from "$lib/componentes/Icono.svelte";
  import { irA } from "$lib/navegacion.svelte";
  import { MAX_NOMBRE, nombreValido, textoFotos } from "./etiquetas";
  import { buscarEtiqueta, cargarEtiquetas, pedirBorrar, renombrar } from "./estado.svelte";

  let { id, version = 0 }: { id: number; version?: number } = $props();

  const etiqueta = $derived(buscarEtiqueta(id));
  let error = $state("");
  let editando = $state(false);
  let borrador = $state("");
  let menuAbierto = $state(false);
  let campo = $state<HTMLInputElement>();

  // Al abrir y cada vez que cambian sus fotos (`version`).
  $effect(() => {
    void version;
    cargarEtiquetas();
  });

  async function empezarAEditar() {
    if (!etiqueta) return;
    menuAbierto = false;
    borrador = etiqueta.name;
    editando = true;
    await tick();
    campo?.select();
  }

  async function guardar() {
    if (!editando || !etiqueta) return;
    editando = false;
    const nombre = nombreValido(borrador);
    if (!nombre || nombre === etiqueta.name) return;
    error = (await renombrar(etiqueta, nombre)) ?? "";
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

  function alPulsarVentana(e: KeyboardEvent) {
    if (e.key === "Escape" && menuAbierto) {
      menuAbierto = false;
      e.preventDefault();
      e.stopPropagation();
    }
  }
</script>

<svelte:window onkeydowncapture={alPulsarVentana} />

<header class="cabecera">
  <button type="button" class="volver" onclick={() => irA("etiquetas")}>
    <Icono nombre="anterior" tamano={18} />
    <span>Etiquetas</span>
  </button>
  <div class="fila">
    <div class="textos">
      {#if editando}
        <input
          bind:this={campo}
          bind:value={borrador}
          class="titulo"
          maxlength={MAX_NOMBRE}
          aria-label="Nombre de la etiqueta"
          onkeydown={alPulsarCampo}
          onblur={guardar}
        />
      {:else}
        <h1>
          <button type="button" class="titulo" title="Renombrar" onclick={empezarAEditar}>
            {etiqueta?.name ?? ""}
          </button>
        </h1>
      {/if}
      {#if etiqueta}
        <p class="subtitulo">{textoFotos(etiqueta.photos)}</p>
      {/if}
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
    </div>
    <div class="menu">
      <button
        type="button"
        class="puntos"
        aria-label="Más opciones de la etiqueta"
        aria-haspopup="menu"
        aria-expanded={menuAbierto}
        onclick={() => (menuAbierto = !menuAbierto)}
      >
        <Icono nombre="puntos" grosor={3} />
      </button>
      {#if menuAbierto}
        <div class="desplegable" role="menu">
          <button type="button" role="menuitem" onclick={empezarAEditar}>Renombrar</button>
          <button
            type="button"
            role="menuitem"
            class="peligro"
            onclick={() => {
              menuAbierto = false;
              if (etiqueta) pedirBorrar(etiqueta);
            }}
          >
            Borrar etiqueta
          </button>
        </div>
      {/if}
    </div>
  </div>
</header>

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
</style>
