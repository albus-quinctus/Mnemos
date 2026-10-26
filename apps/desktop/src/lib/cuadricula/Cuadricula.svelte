<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import {
    fotos as pedirFotos,
    meses as pedirMeses,
    priorizarMiniaturas,
    type Cursor,
    type EventoMiniaturas,
    type FotoCuadricula,
    type Mes,
  } from "$lib/biblioteca";
  import { miniaturaUrl } from "$lib/media";
  import { disponer, nombreMes, visibles, type Opciones } from "./disposicion";

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
    ancho: Math.max(0, ancho - 48),
    altoFila: 200,
    separacion: 4,
    altoMes: 56,
  });
  const disposicion = $derived(disponer(meses, fotos, opciones));
  const enPantalla = $derived(
    visibles(disposicion.bloques, desplazamiento - MARGEN, desplazamiento + altoVista + MARGEN),
  );

  async function cargarPagina() {
    if (cargando || !quedan) return;
    cargando = true;
    try {
      const pagina = await pedirFotos(siguiente, PAGINA);
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
    pedirMeses()
      .then((m) => {
        meses = m;
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
  class="cuadricula"
  bind:this={contenedor}
  bind:clientWidth={ancho}
  bind:clientHeight={altoVista}
  onscroll={alDesplazar}
>
  {#if error}
    <p class="aviso" role="alert">{error}</p>
  {:else if meses.length === 0}
    <p class="aviso">Esta biblioteca aún no tiene fotos.</p>
  {/if}

  <div class="lienzo" style:height="{disposicion.alto}px">
    {#each enPantalla as bloque (bloque.tipo === "fila" ? `f${fotos[bloque.celdas[0].indice].id}` : `${bloque.tipo}${bloque.mes}`)}
      {#if bloque.tipo === "mes"}
        <h2 class="mes" style:transform="translateY({bloque.y}px)" style:height="{bloque.alto}px">
          {nombreMes(bloque.mes)}
        </h2>
      {:else if bloque.tipo === "fila"}
        <div class="fila" style:transform="translateY({bloque.y}px)" style:height="{bloque.alto}px">
          {#each bloque.celdas as celda (fotos[celda.indice].id)}
            {@const foto = fotos[celda.indice]}
            <div class="celda" style:left="{celda.x}px" style:width="{celda.ancho}px">
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
            </div>
          {/each}
        </div>
      {:else}
        <div class="pendiente" style:transform="translateY({bloque.y}px)" style:height="{bloque.alto}px"></div>
      {/if}
    {/each}
  </div>
</div>

<style>
  .cuadricula {
    position: relative;
    height: 100%;
    overflow-y: auto;
    padding: 0 24px 24px;
    contain: strict;
  }

  .lienzo {
    position: relative;
  }

  .mes,
  .fila,
  .pendiente {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    will-change: transform;
  }

  .mes {
    display: flex;
    align-items: flex-end;
    margin: 0;
    padding-bottom: 12px;
    font-size: 1rem;
    font-weight: 600;
  }

  .celda {
    position: absolute;
    top: 0;
    height: 100%;
    overflow: hidden;
    border-radius: 2px;
    background: var(--hueco);
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
      var(--hueco) 0 200px,
      transparent 200px 204px
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
