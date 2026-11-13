<script lang="ts">
  // Pantalla Álbumes (spec 014): tarjetas con portada, nombre y número de
  // fotos, del álbum modificado más recientemente al más antiguo.
  import Icono from "$lib/componentes/Icono.svelte";
  import { miniaturaUrl } from "$lib/media";
  import { abrirAlbum } from "$lib/navegacion.svelte";
  import { textoAlbumes, textoFotos } from "./albumes";
  import { cargarAlbumes, estadoAlbumes, pedirNuevoAlbum } from "./estado.svelte";
  import NuevoAlbum from "./NuevoAlbum.svelte";

  // Portadas que no se pudieron cargar: se ve el recuadro gris.
  let sinPortada = $state(new Set<number>());

  $effect(() => {
    cargarAlbumes();
  });
</script>

<section class="albumes">
  <header>
    <div>
      <h1>Álbumes</h1>
      {#if estadoAlbumes.cargado && !estadoAlbumes.error}
        <p class="resumen">{textoAlbumes(estadoAlbumes.lista.length)}</p>
      {/if}
    </div>
    <button type="button" class="nuevo" onclick={pedirNuevoAlbum}>
      <Icono nombre="mas" />
      <span>Nuevo álbum</span>
    </button>
  </header>

  {#if estadoAlbumes.error}
    <p class="error" role="alert">{estadoAlbumes.error}</p>
  {:else if estadoAlbumes.cargado && estadoAlbumes.lista.length === 0}
    <div class="vacio">
      <div class="circulo"><Icono nombre="albumes" tamano={34} grosor={1.5} /></div>
      <h2>Aún no tienes álbumes</h2>
      <p>Selecciona fotos y pulsa «Añadir a álbum», o crea uno con «Nuevo álbum».</p>
    </div>
  {:else}
    <ul class="tarjetas">
      {#each estadoAlbumes.lista as album (album.id)}
        <li>
          <button type="button" class="tarjeta" onclick={() => abrirAlbum(album.id)}>
            <span class="portada">
              {#if album.cover !== null && !sinPortada.has(album.id)}
                <img
                  src={miniaturaUrl(album.cover, album.cover_edit_version ?? undefined)}
                  alt=""
                  loading="lazy"
                  onerror={() => (sinPortada = new Set(sinPortada).add(album.id))}
                />
              {:else}
                <Icono nombre="albumes" tamano={44} grosor={1.5} />
              {/if}
            </span>
            <span class="nombre" title={album.name}>{album.name}</span>
            <span class="numero">{textoFotos(album.photos)}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#if estadoAlbumes.creando}
  <NuevoAlbum />
{/if}

<style>
  .albumes {
    padding: 1.75rem 2rem 3rem;
  }

  header {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  header > div {
    flex: 1;
  }

  h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
  }

  .resumen {
    margin: 0.25rem 0 0;
    color: var(--texto-suave);
  }

  .nuevo {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 40px;
    padding: 0 1.1rem 0 0.9rem;
    border: 1px solid var(--borde);
    border-radius: 999px;
    background: var(--superficie);
    color: var(--acento);
    font-weight: 600;
    cursor: pointer;
  }

  .nuevo:hover {
    background: var(--acento-suave);
  }

  .tarjetas {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(196px, 1fr));
    gap: 1.5rem 1.25rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tarjeta {
    display: flex;
    flex-direction: column;
    width: 100%;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
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
  .tarjeta:focus-visible .portada {
    outline: 3px solid var(--acento-suave);
  }

  .tarjeta:focus-visible {
    outline: none;
  }

  .tarjeta:focus-visible .portada {
    outline-color: var(--acento);
  }

  .nombre {
    margin-top: 0.55rem;
    overflow: hidden;
    font-weight: 600;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .numero {
    color: var(--texto-suave);
    font-size: 0.9rem;
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
