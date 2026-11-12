<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick, untrack } from "svelte";
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
    type PaginaFotos,
  } from "$lib/biblioteca";
  import Icono from "$lib/componentes/Icono.svelte";
  import { miniaturaUrl } from "$lib/media";
  import {
    alternarDia,
    avisarFallo,
    cambiarAmbito,
    clicEnFoto,
    haySeleccion,
    seleccion,
    teclaSeleccion,
    todasSeleccionadas,
  } from "$lib/seleccion/seleccion.svelte";
  import { cambios } from "$lib/cambios.svelte";
  import {
    alternarArchivo,
    alternarFavoritos,
    registrarFotos,
  } from "$lib/seleccion/acciones.svelte";
  import { fechas } from "$lib/visor/fechas.svelte";
  import Visor from "$lib/visor/Visor.svelte";
  import BarraMeses from "./BarraMeses.svelte";
  import { diaDe, disponer, nombreDia, visibles, type Opciones } from "./disposicion";

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

  // La selección es de esta vista o de esta búsqueda: cambiarla la pierde.
  $effect(() => {
    const ambito = texto ? { busqueda: texto } : { vista: "fotos" as const };
    untrack(() => cambiarAmbito(ambito));
  });

  // Ids cargados de cada día, para marcar la casilla de su cabecera.
  const idsPorDia = $derived.by(() => {
    const porDia = new Map<string, number[]>();
    for (const foto of fotos) {
      const dia = diaDe(foto);
      const ids = porDia.get(dia);
      if (ids) ids.push(foto.id);
      else porDia.set(dia, [foto.id]);
    }
    return porDia;
  });

  function alPulsarFoto(e: MouseEvent, indice: number) {
    const mods = { ctrl: e.ctrlKey || e.metaKey, mayus: e.shiftKey };
    if (!clicEnFoto(fotos[indice], mods)) abierta = indice;
  }

  function alPulsarCasilla(e: MouseEvent, indice: number) {
    clicEnFoto(fotos[indice], { ctrl: true, mayus: e.shiftKey });
  }

  // Ctrl + A y Esc, salvo con el visor abierto o escribiendo en un campo.
  function alPulsarTecla(e: KeyboardEvent) {
    if (abierta !== null) return;
    const destino = e.target as HTMLElement | null;
    if (destino?.closest("input, textarea, [contenteditable]")) return;
    if (teclaSeleccion(e.key, { ctrl: e.ctrlKey || e.metaKey, mayus: e.shiftKey })) {
      e.preventDefault();
      return;
    }
    // Con selección, F marca favoritos y Mayús + A archiva, como en el visor.
    if (!haySeleccion() || e.ctrlKey || e.metaKey || e.altKey) return;
    if ((e.key === "f" || e.key === "F") && !e.shiftKey) alternarFavoritos();
    else if ((e.key === "a" || e.key === "A") && e.shiftKey) alternarArchivo();
    else return;
    e.preventDefault();
  }

  // Archivar desde el visor: en Fotos la foto deja de estar y el visor pasa a
  // la siguiente (o a la anterior si era la última). En la búsqueda se queda.
  function alArchivarEnVisor(indice: number, archivada: boolean) {
    if (texto || !archivada) return;
    fotos = fotos.toSpliced(indice, 1);
    if (fotos.length === 0) abierta = null;
    else if (indice >= fotos.length) abierta = fotos.length - 1;
  }

  // Foto abierta en el visor, por su posición en `fotos`.
  let abierta = $state<number | null>(null);

  /** Al cerrar el visor, la cuadrícula muestra y enfoca la última foto vista. */
  async function cerrarVisor() {
    const indice = abierta;
    vista = indice === null ? null : (fotos[indice]?.id ?? null);
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

  // Última foto vista en el visor, para enfocarla tras recargar.
  let vista: number | null = null;

  /**
   * Vuelve a pedir los meses y las fotos ya cargadas sin mover el
   * desplazamiento, cuando cambia la fecha de alguna foto (spec 011).
   */
  async function recargar() {
    while (cargando) await new Promise((r) => setTimeout(r, 50));
    cargando = true;
    const cuantas = Math.max(fotos.length, 1);
    try {
      const nuevosMeses = texto ? await buscarMeses(texto) : await pedirMeses();
      let nuevas: FotoCuadricula[] = [];
      let cursor: Cursor | null = null;
      do {
        const pagina: PaginaFotos = texto
          ? await buscarFotos(texto, cursor, PAGINA)
          : await pedirFotos(cursor, PAGINA);
        nuevas = nuevas.concat(pagina.photos);
        cursor = pagina.next;
      } while (cursor !== null && nuevas.length < cuantas);
      const arriba = contenedor?.scrollTop ?? 0;
      meses = nuevosMeses;
      fotos = nuevas;
      siguiente = cursor;
      quedan = cursor !== null;
      await tick();
      if (contenedor) {
        contenedor.scrollTop = arriba;
        desplazamiento = contenedor.scrollTop;
      }
    } catch (e) {
      error = String(e);
    } finally {
      cargando = false;
    }
    const indice = fotos.findIndex((f) => f.id === vista);
    if (indice >= 0 && abierta === null) {
      await tick();
      contenedor?.querySelector<HTMLElement>(`[data-indice="${indice}"]`)?.focus();
    }
  }

  // Al cambiar alguna fecha a mano, se recarga. La primera vez solo se anota
  // la versión: la carga inicial ya la hace `onMount`.
  // Lo mismo al marcar favoritos o archivar (spec 006).
  let versionFechas = fechas.version;
  let versionCambios = cambios.version;
  $effect(() => {
    if (fechas.version === versionFechas && cambios.version === versionCambios) return;
    versionFechas = fechas.version;
    versionCambios = cambios.version;
    if (listo) recargar();
  });

  // La barra de selección pregunta si las seleccionadas ya son favoritas.
  const porId = $derived(new Map(fotos.map((f) => [f.id, f])));
  onMount(() => registrarFotos((id) => porId.get(id)));

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

<svelte:window onkeydown={alPulsarTecla} />

<div
  class="vista"
  role="presentation"
  onkeydown={(e) => {
    // Con selección, Esc solo la quita (lo hace `alPulsarTecla`).
    if (e.key === "Escape" && abierta === null && !haySeleccion()) onSalir?.();
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

  <div class="lienzo" class:seleccionando={seleccion.ids.size > 0} style:height="{disposicion.alto}px">
    {#each enPantalla as bloque (bloque.tipo === "fila" ? `f${fotos[bloque.celdas[0].indice].id}` : `p${bloque.mes}`)}
      {#if bloque.tipo === "fila"}
        <div class="fila" style:transform="translateY({bloque.y}px)" style:height="{bloque.alto}px">
          {#each bloque.titulos as titulo (titulo.dia)}
            {@const completo = todasSeleccionadas(idsPorDia.get(titulo.dia) ?? [])}
            <h2 class="dia" style:left="{titulo.x}px" style:height="{bloque.altoTitulo}px">
              <button
                type="button"
                class="casilla-dia"
                class:marcada={completo}
                role="checkbox"
                aria-checked={completo}
                aria-label="Seleccionar el día {nombreDia(titulo.dia)}"
                onclick={() => alternarDia(titulo.dia).catch(avisarFallo)}
              >
                {#if completo}<Icono nombre="marca" tamano={14} grosor={3} />{/if}
              </button>
              {nombreDia(titulo.dia)}
            </h2>
          {/each}
          {#each bloque.celdas as celda (fotos[celda.indice].id)}
            {@const foto = fotos[celda.indice]}
            {@const marcada = seleccion.ids.has(foto.id)}
            <button
              type="button"
              class="celda"
              class:marcada
              data-indice={celda.indice}
              aria-label={foto.name}
              aria-pressed={seleccion.ids.size > 0 ? marcada : undefined}
              onclick={(e) => alPulsarFoto(e, celda.indice)}
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
              {#if foto.favorite}
                <span class="insignia favorita" title="En Favoritos">
                  <Icono nombre="estrella" tamano={18} relleno />
                </span>
              {/if}
              {#if texto && foto.archived}
                <span class="insignia archivada" title="Archivada">
                  <Icono nombre="archivo" tamano={16} />
                </span>
              {/if}
            </button>
            <!-- Fuera del botón de la celda: un botón no puede ir dentro de otro. -->
            <button
              type="button"
              class="casilla"
              class:marcada
              tabindex="-1"
              role="checkbox"
              aria-checked={marcada}
              aria-label="Seleccionar {foto.name}"
              onclick={(e) => alPulsarCasilla(e, celda.indice)}
              style:left="{celda.x + 8}px"
              style:top="{bloque.altoTitulo + 8}px"
            >
              {#if marcada}<Icono nombre="marca" tamano={16} grosor={3} />{/if}
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
    onArchivada={alArchivarEnVisor}
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

  /* Seleccionada: miniatura algo reducida dentro de un marco de acento. */
  .celda.marcada {
    background: var(--acento-suave);
  }

  .celda.marcada img,
  .celda.marcada .sin-vista {
    transform: scale(0.88);
    border-radius: 6px;
  }

  .celda.marcada::after {
    content: "";
    position: absolute;
    inset: 0;
    border: 3px solid var(--acento);
    border-radius: 2px;
    pointer-events: none;
  }

  /* El círculo sale al pasar el ratón y siempre mientras hay selección. */
  .casilla {
    position: absolute;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 2px solid #fff;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.18);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
    color: var(--acento-texto);
    cursor: pointer;
    opacity: 0;
    transition: opacity 80ms ease-out;
  }

  .celda:hover + .casilla,
  .casilla:hover,
  .seleccionando .casilla {
    opacity: 1;
  }

  .casilla.marcada {
    border-color: var(--acento);
    background: var(--acento);
  }

  .insignia {
    position: absolute;
    bottom: 8px;
    display: flex;
    color: #fff;
    pointer-events: none;
  }

  .favorita {
    left: 8px;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.6));
  }

  .archivada {
    right: 8px;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.45);
  }

  .casilla-dia {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    margin-right: 0.5rem;
    padding: 0;
    border: 2px solid var(--texto-suave);
    border-radius: 50%;
    background: transparent;
    color: var(--acento-texto);
    cursor: pointer;
    /* Oculta no ocupa sitio: la fecha queda alineada con las fotos. */
    display: none;
  }

  .dia:hover .casilla-dia,
  .casilla-dia:focus-visible,
  .casilla-dia.marcada,
  .seleccionando .casilla-dia {
    display: inline-flex;
  }

  .casilla-dia.marcada {
    border-color: var(--acento);
    background: var(--acento);
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
