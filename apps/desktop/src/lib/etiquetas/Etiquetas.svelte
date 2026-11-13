<script lang="ts">
  // Pantalla Etiquetas (spec 016): tarjetas con la foto más reciente, el
  // nombre y el número de fotos, por orden alfabético. Cada tarjeta tiene un
  // menú ⋮ (o clic derecho) para renombrarla en el sitio o borrarla.
  import { tick } from "svelte";
  import type { Etiqueta } from "$lib/biblioteca";
  import Icono from "$lib/componentes/Icono.svelte";
  import { miniaturaUrl } from "$lib/media";
  import { abrirEtiqueta } from "$lib/navegacion.svelte";
  import { MAX_NOMBRE, nombreValido, textoEtiquetas, textoFotos } from "./etiquetas";
  import { cargarEtiquetas, crear, estadoEtiquetas, pedirBorrar, renombrar } from "./estado.svelte";

  // Portadas que no se pudieron cargar: se ve el recuadro gris.
  let sinPortada = $state(new Set<number>());
  let menuDe = $state<number | null>(null);
  let renombrando = $state<number | null>(null);
  let borrador = $state("");
  let nuevo = $state("");
  let error = $state("");
  let campoNuevo = $state<HTMLInputElement>();
  let campoNombre = $state<HTMLInputElement>();

  $effect(() => {
    cargarEtiquetas();
  });

  async function abrirNueva() {
    estadoEtiquetas.creando = true;
    nuevo = "";
    error = "";
    await tick();
    campoNuevo?.focus();
  }

  async function guardarNueva(e?: SubmitEvent) {
    e?.preventDefault();
    const nombre = nombreValido(nuevo);
    if (!nombre) {
      error = `El nombre debe tener entre 1 y ${MAX_NOMBRE} caracteres.`;
      return;
    }
    error = (await crear(nombre)) ?? "";
  }

  async function empezarARenombrar(etiqueta: Etiqueta) {
    menuDe = null;
    borrador = etiqueta.name;
    renombrando = etiqueta.id;
    await tick();
    campoNombre?.select();
  }

  async function guardarNombre(etiqueta: Etiqueta) {
    if (renombrando !== etiqueta.id) return;
    renombrando = null;
    const nombre = nombreValido(borrador);
    if (!nombre || nombre === etiqueta.name) return;
    error = (await renombrar(etiqueta, nombre)) ?? "";
  }

  function teclaNombre(e: KeyboardEvent, etiqueta: Etiqueta) {
    if (e.key === "Enter") {
      e.preventDefault();
      guardarNombre(etiqueta);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      renombrando = null;
    }
  }

  function teclaNueva(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      estadoEtiquetas.creando = false;
    }
  }

  function abrirMenu(e: MouseEvent, id: number) {
    e.preventDefault();
    e.stopPropagation();
    menuDe = menuDe === id ? null : id;
  }

  function alPulsarVentana(e: KeyboardEvent) {
    if (e.key === "Escape" && menuDe !== null) {
      menuDe = null;
      e.preventDefault();
      e.stopPropagation();
    }
  }
</script>

<svelte:window
  onkeydowncapture={alPulsarVentana}
  onclick={() => (menuDe = null)}
/>

<section class="etiquetas">
  <header>
    <h1>Etiquetas</h1>
    {#if estadoEtiquetas.cargado && !estadoEtiquetas.error}
      <span class="resumen">· {textoEtiquetas(estadoEtiquetas.lista.length)}</span>
    {/if}
    <button type="button" class="nueva" onclick={abrirNueva}>
      <Icono nombre="mas" tamano={18} grosor={2.2} />
      <span>Nueva etiqueta</span>
    </button>
  </header>

  {#if estadoEtiquetas.creando}
    <form class="crear" onsubmit={guardarNueva}>
      <input
        bind:this={campoNuevo}
        bind:value={nuevo}
        maxlength={MAX_NOMBRE}
        placeholder="Nombre de la etiqueta"
        aria-label="Nombre de la nueva etiqueta"
        onkeydown={teclaNueva}
      />
      <button type="submit" class="texto azul">Crear</button>
      <button type="button" class="texto" onclick={() => (estadoEtiquetas.creando = false)}>
        Cancelar
      </button>
    </form>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if estadoEtiquetas.error}
    <p class="error" role="alert">{estadoEtiquetas.error}</p>
  {:else if estadoEtiquetas.cargado && estadoEtiquetas.lista.length === 0 && !estadoEtiquetas.creando}
    <div class="vacio">
      <div class="circulo"><Icono nombre="etiquetas" tamano={34} grosor={1.5} /></div>
      <h2>Aún no hay etiquetas</h2>
      <p>Selecciona fotos y pulsa «Etiquetar», o crea una con «Nueva etiqueta».</p>
    </div>
  {:else}
    <ul class="tarjetas">
      {#each estadoEtiquetas.lista as etiqueta (etiqueta.id)}
        <li class:con-menu={menuDe === etiqueta.id}>
          <button
            type="button"
            class="tarjeta"
            onclick={() => abrirEtiqueta(etiqueta.id)}
            oncontextmenu={(e) => abrirMenu(e, etiqueta.id)}
          >
            <span class="portada">
              {#if etiqueta.cover !== null && !sinPortada.has(etiqueta.id)}
                <img
                  src={miniaturaUrl(etiqueta.cover, etiqueta.cover_edit_version ?? undefined)}
                  alt=""
                  loading="lazy"
                  onerror={() => (sinPortada = new Set(sinPortada).add(etiqueta.id))}
                />
              {:else}
                <Icono nombre="etiquetas" tamano={40} grosor={1.5} />
              {/if}
            </span>
          </button>
          {#if renombrando === etiqueta.id}
            <input
              bind:this={campoNombre}
              bind:value={borrador}
              class="renombrar"
              maxlength={MAX_NOMBRE}
              aria-label="Nombre de la etiqueta"
              onkeydown={(e) => teclaNombre(e, etiqueta)}
              onblur={() => guardarNombre(etiqueta)}
            />
          {:else}
            <span class="nombre" title={etiqueta.name}>{etiqueta.name}</span>
          {/if}
          <span class="numero">{textoFotos(etiqueta.photos)}</span>
          <button
            type="button"
            class="puntos"
            aria-label={`Más opciones de «${etiqueta.name}»`}
            aria-haspopup="menu"
            aria-expanded={menuDe === etiqueta.id}
            onclick={(e) => abrirMenu(e, etiqueta.id)}
          >
            <Icono nombre="puntos" grosor={3} />
          </button>
          {#if menuDe === etiqueta.id}
            <div class="desplegable" role="menu">
              <button type="button" role="menuitem" onclick={() => empezarARenombrar(etiqueta)}>
                <Icono nombre="editar" tamano={18} />
                Renombrar
              </button>
              <button
                type="button"
                role="menuitem"
                class="peligro"
                onclick={() => {
                  menuDe = null;
                  pedirBorrar(etiqueta);
                }}
              >
                <Icono nombre="papelera" tamano={18} />
                Borrar
              </button>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .etiquetas {
    padding: 1.75rem 2rem 3rem;
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-bottom: 1.25rem;
  }

  h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
  }

  .resumen {
    color: var(--texto-suave);
  }

  .nueva {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 40px;
    margin-left: auto;
    padding: 0 1.1rem 0 0.9rem;
    border: 0;
    border-radius: 999px;
    background: var(--acento);
    color: var(--acento-texto);
    font-weight: 600;
    cursor: pointer;
  }

  .crear {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: -0.25rem 0 1.25rem;
  }

  .crear input {
    width: min(320px, 100%);
    height: 40px;
    padding: 0 0.9rem;
    border: 2px solid var(--acento);
    border-radius: 999px;
    background: var(--superficie);
    outline: none;
  }

  .texto {
    height: 40px;
    padding: 0 1rem;
    border: 0;
    border-radius: 999px;
    background: transparent;
    font-weight: 500;
    cursor: pointer;
  }

  .texto:hover {
    background: var(--superficie-alta);
  }

  .texto.azul {
    color: var(--acento);
    font-weight: 600;
  }

  .tarjetas {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(196px, 1fr));
    gap: 1.5rem 1.25rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .tarjeta {
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    cursor: pointer;
  }

  .portada {
    display: flex;
    align-items: center;
    justify-content: center;
    aspect-ratio: 1;
    overflow: hidden;
    border-radius: 12px;
    background: var(--hueco);
    color: var(--texto-suave);
  }

  .portada img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .tarjeta:hover .portada,
  .con-menu .portada {
    outline: 3px solid var(--acento-suave);
  }

  .tarjeta:focus-visible {
    outline: none;
  }

  .tarjeta:focus-visible .portada {
    outline: 3px solid var(--acento);
  }

  .nombre {
    margin-top: 0.55rem;
    overflow: hidden;
    font-weight: 600;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .renombrar {
    height: 34px;
    margin-top: 0.4rem;
    padding: 0 0.6rem;
    border: 2px solid var(--acento);
    border-radius: 6px;
    background: var(--superficie);
    font-weight: 600;
    outline: none;
  }

  .numero {
    color: var(--texto-suave);
    font-size: 0.9rem;
  }

  .puntos {
    position: absolute;
    top: 8px;
    right: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: 0;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.55);
    color: #fff;
    cursor: pointer;
    opacity: 0;
  }

  li:hover .puntos,
  .puntos:focus-visible,
  .con-menu .puntos {
    opacity: 1;
  }

  .desplegable {
    position: absolute;
    top: 48px;
    right: 8px;
    z-index: 30;
    display: flex;
    flex-direction: column;
    width: 12rem;
    padding: 0.35rem;
    border: 1px solid var(--borde);
    border-radius: 12px;
    background: var(--superficie);
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.18);
  }

  .desplegable button {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    height: 40px;
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

  .vacio {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    margin-top: 6rem;
    color: var(--texto-suave);
    text-align: center;
  }

  .circulo {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 72px;
    height: 72px;
    border-radius: 50%;
    background: var(--acento-suave);
    color: var(--acento);
  }

  .vacio h2 {
    margin: 0;
    color: var(--texto);
    font-size: 1.1rem;
    font-weight: 600;
  }

  .vacio p {
    margin: 0;
  }

  .error {
    color: var(--error);
  }
</style>
