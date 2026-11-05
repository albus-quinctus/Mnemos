<script lang="ts">
  // Vista Duplicados (spec 010): fotos idénticas byte a byte, agrupadas por
  // huella. Solo enseña: no hay selección ni botones de borrar.
  import {
    duplicadosGrupos,
    duplicadosResumen,
    type CursorDuplicados,
    type GrupoDuplicado,
    type ResumenDuplicados,
  } from "$lib/biblioteca";
  import Icono from "$lib/componentes/Icono.svelte";
  import { miniaturaUrl } from "$lib/media";
  import { fechas } from "$lib/visor/fechas.svelte";
  import { tamano } from "$lib/visor/formato";
  import Visor from "$lib/visor/Visor.svelte";
  import { fechaCopia, fotosDelGrupo, separarRuta, textoResumen } from "./duplicados";

  const PAGINA = 50;

  let resumen = $state<ResumenDuplicados | null>(null);
  let grupos = $state<GrupoDuplicado[]>([]);
  let siguiente = $state<CursorDuplicados | null>(null);
  let cargando = $state(false);
  let error = $state("");
  // Grupo y copia abiertos en el visor.
  let abierta = $state<{ grupo: number; copia: number } | null>(null);
  // Miniaturas que no se pudieron cargar: se muestra el marcador.
  let sinMiniatura = $state(new Set<string>());

  async function cargar() {
    cargando = true;
    error = "";
    try {
      const [r, pagina] = await Promise.all([duplicadosResumen(), duplicadosGrupos(null, PAGINA)]);
      resumen = r;
      grupos = pagina.groups;
      siguiente = pagina.next;
    } catch (e) {
      error = String(e);
    } finally {
      cargando = false;
    }
  }

  async function verMas() {
    if (!siguiente || cargando) return;
    cargando = true;
    try {
      const pagina = await duplicadosGrupos(siguiente, PAGINA);
      grupos = [...grupos, ...pagina.groups];
      siguiente = pagina.next;
    } catch (e) {
      error = String(e);
    } finally {
      cargando = false;
    }
  }

  // Al principio y cada vez que el visor cambia una fecha a mano (spec 011).
  $effect(() => {
    void fechas.version;
    cargar();
  });

  function sinVista(hash: string) {
    sinMiniatura = new Set(sinMiniatura).add(hash);
  }
</script>

<section class="duplicados">
  <h1>Duplicados</h1>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {:else if resumen && resumen.groups === 0}
    <div class="vacio">
      <Icono nombre="duplicados" tamano={48} />
      <h2>No hay fotos repetidas</h2>
      <p>Ninguna foto de la biblioteca tiene otra copia idéntica.</p>
    </div>
  {:else if resumen}
    <p class="resumen">{textoResumen(resumen)}</p>
    <p class="nota">
      <Icono nombre="info" />
      MNEMOS solo te enseña las fotos idénticas byte a byte. No borra ni mueve nada.
    </p>

    {#each grupos as grupo, g (grupo.content_hash)}
      <article class="grupo">
        <div class="mini">
          {#if !sinMiniatura.has(grupo.content_hash)}
            <img
              src={miniaturaUrl(grupo.copies[0].id)}
              alt=""
              loading="lazy"
              draggable="false"
              onerror={() => sinVista(grupo.content_hash)}
            />
          {:else}
            <span>Sin vista previa</span>
          {/if}
        </div>
        <div class="datos">
          <p class="cab">
            {grupo.copies.length} copias
            <span>· {tamano(grupo.copies[0].size_bytes)} cada una</span>
          </p>
          <ul>
            {#each grupo.copies as copia, c (copia.id)}
              {@const ruta = separarRuta(copia.rel_path)}
              <li>
                <button
                  type="button"
                  title={copia.rel_path}
                  onclick={() => (abierta = { grupo: g, copia: c })}
                >
                  <span class="ruta"><span class="carpeta">{ruta.carpeta}</span>{ruta.nombre}</span>
                  {#if copia.favorite}<span class="marca favorita">Favorita</span>{/if}
                  {#if copia.archived}<span class="marca">Archivada</span>{/if}
                  {#if !copia.exists}<span class="marca desaparecida">Desaparecida</span>{/if}
                  <span class="fecha">{fechaCopia(copia)}</span>
                </button>
              </li>
            {/each}
          </ul>
        </div>
      </article>
    {/each}

    {#if siguiente}
      <button type="button" class="mas" onclick={verMas} disabled={cargando}>
        Ver más grupos
      </button>
    {/if}
  {/if}
</section>

{#if abierta !== null}
  {@const grupo = grupos[abierta.grupo]}
  <Visor
    fotos={fotosDelGrupo(grupo)}
    indice={abierta.copia}
    quedan={false}
    onCambiar={(i) => abierta && (abierta = { grupo: abierta.grupo, copia: i })}
    onCerrar={() => (abierta = null)}
    onPedirMas={() => {}}
  />
{/if}

<style>
  .duplicados {
    max-width: 72rem;
    padding: 1.5rem 2rem 3rem;
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

  .nota {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 1rem 0 1.25rem;
    padding: 0.6rem 0.9rem;
    border-radius: var(--radio);
    background: var(--acento-suave);
    font-size: 0.9rem;
  }

  .grupo {
    display: flex;
    gap: 1.25rem;
    margin-bottom: 0.75rem;
    padding: 1rem;
    border: 1px solid var(--borde);
    border-radius: var(--radio);
    background: var(--superficie);
  }

  .mini {
    display: grid;
    place-items: center;
    flex: none;
    width: 112px;
    height: 112px;
    overflow: hidden;
    border-radius: 6px;
    background: var(--hueco);
    color: var(--texto-suave);
    font-size: 0.75rem;
    text-align: center;
  }

  .mini img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .datos {
    flex: 1;
    min-width: 0;
  }

  .cab {
    margin: 0 0 0.35rem;
    font-weight: 600;
  }

  .cab span {
    color: var(--texto-suave);
    font-weight: 400;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li button {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: calc(100% + 1.2rem);
    min-height: 36px;
    margin: 0 -0.6rem;
    padding: 0 0.6rem;
    border: 0;
    border-radius: 6px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  li button:hover {
    background: var(--superficie-alta);
  }

  .ruta {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .carpeta,
  .fecha {
    color: var(--texto-suave);
  }

  .fecha {
    font-size: 0.9rem;
    white-space: nowrap;
  }

  .marca {
    padding: 0.1rem 0.5rem;
    border: 1px solid var(--borde);
    border-radius: 999px;
    color: var(--texto-suave);
    font-size: 0.78rem;
    white-space: nowrap;
  }

  .favorita {
    border-color: var(--acento);
    color: var(--acento);
  }

  .desaparecida {
    border-color: var(--error);
    color: var(--error);
  }

  .vacio {
    margin-top: 4rem;
    color: var(--texto-suave);
    text-align: center;
  }

  .vacio h2 {
    color: var(--texto);
    font-size: 1.15rem;
    font-weight: 600;
  }

  .mas {
    min-height: var(--objetivo);
    padding: 0 1.2rem;
    border: 1px solid var(--borde);
    border-radius: var(--radio);
    background: var(--superficie);
    cursor: pointer;
  }

  .error {
    color: var(--error);
  }
</style>
