<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick } from "svelte";
  import {
    buscarFotos,
    buscarMeses,
    fotos as pedirFotos,
    meses as pedirMeses,
    priorizarMiniaturas,
    type Cursor,
    type EventoMiniaturas,
    type FotoCuadricula,
    type Mes,
  } from "$lib/biblioteca";
  import { miniaturaUrl } from "$lib/media";
  import Visor from "$lib/visor/Visor.svelte";
  import BarraMeses from "./BarraMeses.svelte";
  import { disponer, nombreDia, visibles, type Opciones } from "./disposicion";

  // Con `texto`, la cuadrícula muestra solo los resultados de esa búsqueda.
  // `onSalir` se llama con Esc cuando el visor no está abierto.
  let { texto = "", onSalir }: { texto?: string; onSalir?: () => void } = $props();

  const PAGINA = 500;
  // Margen por encima y por debajo de la pantalla que también se pinta, para
  // que al desplazarse rápido no se vean huecos.
  const MARGEN = 1200;

  let meses = $state<Mes[]>([]);
  // Se guardan sin reactividad profunda: son muchas y solo se añaden.
  let fotos = $state.raw<FotoCuadricula[]>([]);
  let siguiente: Cursor | null = null;
  let quedan = $state(true);
  let cargando = false;
  let error = $state("");
  // Ya han llegado los meses: hasta entonces no se dice que no hay fotos.
  let listo = $state(false);
  const total = $derived(meses.reduce((suma, m) => suma + m.photos, 0));

  // Versión de la miniatura de cada foto: sube cuando la cola la termina, para
  // volver a pedir la que antes dio 404. Las cargadas no se tocan.
  let versiones = $state.raw(new Map<number, number>());
  let fallidas = $state.raw(new Set<number>());
  let cargadas = $state.raw(new Set<number>());

  let contenedor = $state<HTMLDivElement>();
  let ancho = $state(0);
  let altoVista = $state(0);
  let desplazamiento = $state(0);

  const opciones = $derived<Opciones>({
    ancho: Math.max(0, ancho - 24 - 64),
    altoFila: 150,
    separacion: 4,
    separacionDias: 16,
    altoTitulo: 44,
    anchoMinDia: 140,
  });
  const disposicion = $derived(disponer(meses, fotos, opciones));
  const enPantalla = $derived(
    visibles(disposicion.bloques, desplazamiento - MARGEN, desplazamiento + altoVista + MARGEN),
  );

  // Foto abierta en el visor, por su posición en `fotos`.
  let abierta = $state<number | null>(null);

  /** Al cerrar el visor, la cuadrícula muestra y enfoca la última foto vista. */
  async function cerrarVisor() {
    const indice = abierta;
    abierta = null;
    if (indice === null || !contenedor) return;
    const bloque = disposicion.bloques.find(
      (b) => b.tipo === "fila" && b.celdas.some((c) => c.indice === indice),
    );
    if (bloque && (bloque.y < desplazamiento || bloque.y + bloque.alto > desplazamiento + altoVista)) {
      contenedor.scrollTo({ top: Math.max(0, bloque.y - altoVista / 3) });
      desplazamiento = contenedor.scrollTop;
    }
    await tick();
    contenedor.querySelector<HTMLElement>(`[data-indice="${indice}"]`)?.focus();
  }

  async function cargarPagina() {
    if (cargando || !quedan) return;
    cargando = true;
    try {
      const pagina = texto
        ? await buscarFotos(texto, siguiente, PAGINA)
        : await pedirFotos(siguiente, PAGINA);
      fotos = fotos.concat(pagina.photos);
      siguiente = pagina.next;
      quedan = pagina.next !== null;
    } catch (e) {
      error = String(e);
      quedan = false;
    } finally {
      cargando = false;
    }
  }

  // Mientras haya un tramo sin cargar a la vista, se piden más páginas.
  $effect(() => {
    if (enPantalla.some((b) => b.tipo === "pendiente")) cargarPagina();
  });

  // Las miniaturas que se ven pasan delante en la cola, con una pequeña espera
  // para no avisar al núcleo en cada evento de desplazamiento.
  let temporizador: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const ids = enPantalla.flatMap((b) =>
      b.tipo === "fila" ? b.celdas.map((c) => fotos[c.indice].id) : [],
    );
    clearTimeout(temporizador);
    temporizador = setTimeout(() => {
      const faltan = ids.filter((id) => !cargadas.has(id) && !fallidas.has(id));
      if (faltan.length > 0) priorizarMiniaturas(faltan).catch(() => {});
    }, 150);
  });

  function alEvento(evento: EventoMiniaturas) {
    if (evento.tipo === "ready") {
      const nuevas = new Map(versiones);
      for (const id of evento.photo_ids) nuevas.set(id, (nuevas.get(id) ?? 0) + 1);
      versiones = nuevas;
    } else if (evento.tipo === "failed") {
      fallidas = new Set([...fallidas, ...evento.photo_ids]);
    }
  }

  function url(id: number): string {
    const version = versiones.get(id);
    return version ? `${miniaturaUrl(id)}?v=${version}` : miniaturaUrl(id);
  }

  function alCargar(id: number) {
    if (!cargadas.has(id)) cargadas = new Set(cargadas).add(id);
  }

  let pendienteDeDibujo = false;
  function alDesplazar() {
    if (pendienteDeDibujo) return;
    pendienteDeDibujo = true;
    requestAnimationFrame(() => {
      pendienteDeDibujo = false;
      desplazamiento = contenedor?.scrollTop ?? 0;
    });
  }

  onMount(() => {
    let dejarDeEscuchar: (() => void) | undefined;
    listen<EventoMiniaturas>("miniaturas", (e) => alEvento(e.payload))
      .then((f) => (dejarDeEscuchar = f))
      .catch(() => {});
    (texto ? buscarMeses(texto) : pedirMeses())
      .then((m) => {
        meses = m;
        listo = true;
        quedan = m.length > 0;
        return cargarPagina();
      })
      .catch((e) => (error = String(e)));
    return () => {
      dejarDeEscuchar?.();
      clearTimeout(temporizador);
    };
  });
</script>

<div
  class="vista"
  role="presentation"
  onkeydown={(e) => {
    if (e.key === "Escape" && abierta === null) onSalir?.();
  }}
>
{#if texto && listo && !error && total > 0}
  <p class="resumen">
    {total.toLocaleString("es")}
    {total === 1 ? "foto" : "fotos"} para «{texto}»
  </p>
{/if}
<div class="marco">
<div
  id="cuadricula"
  class="cuadricula"
  bind:this={contenedor}
  bind:clientWidth={ancho}
  bind:clientHeight={altoVista}
  onscroll={alDesplazar}
>
  {#if error}
    <p class="aviso" role="alert">{error}</p>
  {:else if listo && meses.length === 0}
    {#if texto}
      <div class="sin-resultados">
        <p class="titulo">No hay fotos para «{texto}»</p>
        <p>
          Se busca en el nombre del archivo, en sus carpetas y en la fecha: por ejemplo
          «playa», «marzo 2024» o «26/10».
        </p>
      </div>
    {:else}
      <p class="aviso">Esta biblioteca aún no tiene fotos.</p>
    {/if}
  {/if}

  <div class="lienzo" style:height="{disposicion.alto}px">
    {#each enPantalla as bloque (bloque.tipo === "fila" ? `f${fotos[bloque.celdas[0].indice].id}` : `p${bloque.mes}`)}
      {#if bloque.tipo === "fila"}
        <div class="fila" style:transform="translateY({bloque.y}px)" style:height="{bloque.alto}px">
          {#each bloque.titulos as titulo (titulo.dia)}
            <h2 class="dia" style:left="{titulo.x}px" style:height="{bloque.altoTitulo}px">
              {nombreDia(titulo.dia)}
            </h2>
          {/each}
          {#each bloque.celdas as celda (fotos[celda.indice].id)}
            {@const foto = fotos[celda.indice]}
            <button
              type="button"
              class="celda"
              data-indice={celda.indice}
              aria-label={foto.name}
              onclick={() => (abierta = celda.indice)}
              style:left="{celda.x}px"
              style:top="{bloque.altoTitulo}px"
              style:width="{celda.ancho}px"
              style:height="{bloque.alto - bloque.altoTitulo}px"
            >
              {#if foto.thumbnail_failed || fallidas.has(foto.id)}
                <div class="sin-vista">
                  <span>Sin vista previa</span>
                  <span class="nombre">{foto.name}</span>
                </div>
              {:else}
                {#key versiones.get(foto.id)}
                  <img
                    src={url(foto.id)}
                    alt={foto.name}
                    decoding="async"
                    draggable="false"
                    class:lista={cargadas.has(foto.id)}
                    onload={() => alCargar(foto.id)}
                  />
                {/key}
              {/if}
            </button>
          {/each}
        </div>
      {:else}
        <div class="pendiente" style:transform="translateY({bloque.y}px)" style:height="{bloque.alto}px"></div>
      {/if}
    {/each}
  </div>
</div>
  {#if disposicion.marcas.length > 0}
    <BarraMeses
      marcas={disposicion.marcas}
      alto={disposicion.alto}
      {altoVista}
      {desplazamiento}
      onIr={(y) => contenedor?.scrollTo({ top: y })}
    />
  {/if}
</div>
</div>

{#if abierta !== null}
  <Visor
    {fotos}
    indice={abierta}
    {quedan}
    onCambiar={(i) => (abierta = i)}
    onCerrar={cerrarVisor}
    onPedirMas={cargarPagina}
  />
{/if}

<style>
  .vista {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .marco {
    position: relative;
    flex: 1;
    min-height: 0;
  }

  .resumen {
    margin: 0;
    padding: 1rem 64px 0.25rem 24px;
    color: var(--texto-suave);
    font-size: 0.9rem;
  }

  .sin-resultados {
    max-width: 32rem;
    padding: 3rem 0;
    color: var(--texto-suave);
  }

  .sin-resultados .titulo {
    margin-bottom: 0.5rem;
    color: var(--texto);
    font-size: 1.1rem;
    font-weight: 500;
  }

  .cuadricula {
    position: relative;
    height: 100%;
    overflow-y: auto;
    padding: 0 64px 24px 24px;
    contain: strict;
    /* La barra de meses sustituye a la del sistema. */
    scrollbar-width: none;
  }

  .cuadricula::-webkit-scrollbar {
    display: none;
  }

  .lienzo {
    position: relative;
  }

  .fila,
  .pendiente {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    will-change: transform;
  }

  .dia {
    position: absolute;
    top: 0;
    display: flex;
    align-items: center;
    margin: 0;
    font-size: 0.9rem;
    font-weight: 500;
    white-space: nowrap;
  }

  .celda {
    position: absolute;
    padding: 0;
    border: 0;
    overflow: hidden;
    border-radius: 2px;
    background: var(--hueco);
    cursor: pointer;
  }

  .celda:focus-visible {
    outline-offset: -2px;
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity 120ms ease-out;
    user-select: none;
  }

  img.lista {
    opacity: 1;
  }

  .pendiente {
    background: repeating-linear-gradient(
      to bottom,
      var(--hueco) 0 150px,
      transparent 150px 154px
    );
    opacity: 0.6;
  }

  .sin-vista {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    height: 100%;
    padding: 8px;
    color: var(--texto-suave);
    font-size: 0.8rem;
    text-align: center;
  }

  .nombre {
    max-width: 100%;
    overflow: hidden;
    font-family: var(--fuente-mono);
    font-size: 0.75rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .aviso {
    padding: 2rem 0;
    color: var(--texto-suave);
  }
</style>
