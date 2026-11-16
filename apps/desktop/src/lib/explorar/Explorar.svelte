<script lang="ts">
  // Pantalla Explorar (spec 021): Recuerdos de esta semana en otros años,
  // colecciones automáticas y las etiquetas con más fotos. Cada tarjeta abre
  // sus fotos en la cuadrícula de siempre.
  import type { Recuerdo, TarjetaColeccion } from "$lib/biblioteca";
  import Icono from "$lib/componentes/Icono.svelte";
  import { miniaturaUrl } from "$lib/media";
  import { abrirEnExplorar, abrirEtiqueta, irA } from "$lib/navegacion.svelte";
  import { textoEtiquetas, textoFotos } from "$lib/etiquetas/etiquetas";
  import { duracion } from "$lib/visor/formato";
  import {
    COLECCIONES,
    haceAnios,
    textoElementos,
    textoRango,
    ultimaImportacion,
  } from "./explorar";
  import { cargarExplorar, estadoExplorar, sinFilas } from "./estado.svelte";
  import Tira from "./Tira.svelte";

  let { version = 0 }: { version?: number } = $props();

  // Portadas que no se pudieron cargar, por clave: se ve el recuadro gris.
  let sinPortada = $state(new Set<string>());
  const fallo = (clave: string) => (sinPortada = new Set(sinPortada).add(clave));

  // Al entrar y cada vez que cambian datos de fotos (`version`).
  $effect(() => {
    void version;
    cargarExplorar();
  });

  function abrirRecuerdo(r: Recuerdo) {
    abrirEnExplorar({
      vista: { rango: { desde: r.desde, hasta: r.hasta } },
      titulo: haceAnios(r.years_ago),
      detalle: textoRango(r.desde, r.hasta),
    });
  }

  function abrirColeccion(c: TarjetaColeccion) {
    abrirEnExplorar({
      vista: { coleccion: c.collection },
      titulo: COLECCIONES[c.collection].nombre,
    });
  }

  const subtituloColeccion = (c: TarjetaColeccion) =>
    c.last_import !== null ? ultimaImportacion(c.last_import) : textoElementos(c.photos);
</script>

<section class="explorar">
  <h1>Explorar</h1>
  <p class="resumen">Recuerdos y colecciones de tu biblioteca</p>

  {#if estadoExplorar.error}
    <p class="error" role="alert">{estadoExplorar.error}</p>
  {:else if estadoExplorar.datos && sinFilas(estadoExplorar.datos)}
    <div class="vacio">
      <div class="circulo"><Icono nombre="explorar" tamano={34} grosor={1.5} /></div>
      <h2>Aún no hay nada que explorar</h2>
      <p>Aquí saldrán tus recuerdos, vídeos, capturas y panorámicas en cuanto los haya.</p>
    </div>
  {:else if estadoExplorar.datos}
    {@const datos = estadoExplorar.datos}
    {#if datos.memories.length > 0}
      <Tira titulo="Recuerdos" nota="Esta semana en otros años">
        {#each datos.memories as r (r.years_ago)}
          {@const clave = `r${r.years_ago}`}
          <li>
            <button
              type="button"
              class="tarjeta recuerdo"
              aria-label="{haceAnios(r.years_ago)}, {textoRango(r.desde, r.hasta)}, {textoElementos(r.photos)}"
              onclick={() => abrirRecuerdo(r)}
            >
              <span class="portada">
                {#if !sinPortada.has(clave)}
                  <img
                    src={miniaturaUrl(r.cover, r.cover_edit_version ?? undefined)}
                    alt=""
                    loading="lazy"
                    onerror={() => fallo(clave)}
                  />
                {/if}
                <span class="velo"></span>
                <span class="texto">
                  <b>{haceAnios(r.years_ago)}</b>
                  <span>{textoRango(r.desde, r.hasta)}</span>
                </span>
              </span>
            </button>
          </li>
        {/each}
      </Tira>
    {/if}

    {#if datos.collections.length > 0}
      <Tira titulo="Colecciones">
        {#each datos.collections as c (c.collection)}
          {@const clave = `c${c.collection}`}
          {@const info = COLECCIONES[c.collection]}
          <li>
            <button type="button" class="tarjeta" onclick={() => abrirColeccion(c)}>
              <span class="portada">
                {#if !sinPortada.has(clave)}
                  <img
                    src={miniaturaUrl(c.cover, c.cover_edit_version ?? undefined)}
                    alt=""
                    loading="lazy"
                    onerror={() => fallo(clave)}
                  />
                {:else}
                  <Icono nombre={info.icono} tamano={44} grosor={1.5} />
                {/if}
                <span class="icono-esquina"><Icono nombre={info.icono} tamano={17} grosor={2} /></span>
                {#if c.cover_duration_ms !== null}
                  <span class="duracion">{duracion(c.cover_duration_ms)}</span>
                {/if}
              </span>
              <span class="nombre">{info.nombre}</span>
              <span class="numero">{subtituloColeccion(c)}</span>
            </button>
          </li>
        {/each}
      </Tira>
    {/if}

    {#if datos.tags.length > 0}
      <Tira titulo="Tus etiquetas">
        {#each datos.tags as t (t.id)}
          {@const clave = `t${t.id}`}
          <li>
            <button type="button" class="tarjeta" onclick={() => abrirEtiqueta(t.id)}>
              <span class="portada">
                {#if t.cover !== null && !sinPortada.has(clave)}
                  <img
                    src={miniaturaUrl(t.cover, t.cover_edit_version ?? undefined)}
                    alt=""
                    loading="lazy"
                    onerror={() => fallo(clave)}
                  />
                {:else}
                  <Icono nombre="etiquetas" tamano={44} grosor={1.5} />
                {/if}
              </span>
              <span class="nombre" title={t.name}>{t.name}</span>
              <span class="numero">{textoFotos(t.photos)}</span>
            </button>
          </li>
        {/each}
        <li>
          <button type="button" class="tarjeta ver-todas" onclick={() => irA("etiquetas")}>
            <span class="portada">
              <Icono nombre="etiquetas" tamano={36} grosor={1.5} />
              <span>Ver todas</span>
            </span>
            <span class="nombre">&nbsp;</span>
            <span class="numero">{textoEtiquetas(datos.tags_total)}</span>
          </button>
        </li>
      </Tira>
    {/if}
  {/if}
</section>

<style>
  .explorar {
    padding: 1.75rem 2rem 3rem;
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

  .tarjeta {
    display: flex;
    flex-direction: column;
    width: 184px;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .portada {
    position: relative;
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

  .tarjeta:hover .portada {
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

  .numero {
    color: var(--texto-suave);
    font-size: 0.9rem;
  }

  .recuerdo {
    width: 200px;
  }

  .recuerdo .portada {
    aspect-ratio: 3 / 4;
  }

  .velo {
    position: absolute;
    inset: 0;
    background: linear-gradient(to top, rgb(0 0 0 / 72%), rgb(0 0 0 / 0%) 55%);
  }

  .texto {
    position: absolute;
    right: 14px;
    bottom: 12px;
    left: 14px;
    display: flex;
    flex-direction: column;
    color: #fff;
  }

  .texto b {
    font-size: 1.15rem;
    font-weight: 650;
  }

  .texto span {
    font-size: 0.88rem;
    opacity: 0.9;
  }

  .icono-esquina {
    position: absolute;
    top: 8px;
    left: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background: rgb(0 0 0 / 55%);
    color: #fff;
  }

  .duracion {
    position: absolute;
    right: 8px;
    bottom: 8px;
    padding: 1px 7px;
    border-radius: 999px;
    background: rgb(0 0 0 / 60%);
    color: #fff;
    font-size: 0.78rem;
    font-weight: 600;
  }

  .ver-todas .portada {
    flex-direction: column;
    gap: 0.4rem;
    background: var(--acento-suave);
    color: var(--acento);
    font-weight: 600;
  }

  .vacio {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    margin-top: 7rem;
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
    max-width: 30rem;
    margin: 0;
  }

  .error {
    margin-top: 1.5rem;
    color: var(--error);
  }
</style>
