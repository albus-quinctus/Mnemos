<script lang="ts">
  // Modo edición del visor (spec 009): recorte, giro y volteo sobre la foto
  // sin editar. Nada se guarda hasta pulsar «Guardar»: la receta va a
  // `.mnemos/ediciones/` y el original nunca se toca.
  import { guardarCopia, guardarEdicion, type Geometria } from "$lib/biblioteca";
  import Icono from "$lib/componentes/Icono.svelte";
  import { originalUrl } from "$lib/media";
  import {
    PROPORCIONES,
    alternarVertical,
    arrastrarEsquina,
    arrastrarLado,
    crearEstado,
    elegirProporcion,
    esIdentidad,
    geometria,
    girar,
    hayCambios,
    mover,
    restablecer,
    tamanoRecorte,
    tamanoVisible,
    tecla,
    voltear,
    type Esquina,
    type EstadoEditor,
    type Lado,
    type Proporcion,
  } from "./editor";

  let {
    id,
    nombre,
    ancho,
    alto,
    base,
    onCerrar,
  }: {
    id: number;
    nombre: string;
    /** Tamaño de la foto ya girada según el EXIF, sin la receta. */
    ancho: number;
    alto: number;
    /** Geometría de la versión que se ve al entrar. */
    base: Geometria;
    /** Al salir: qué se guardó, o null si se canceló. */
    onCerrar: (guardado: { como: "original" } | { como: "copia"; id: number } | null) => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let e = $state<EstadoEditor>(crearEstado(ancho, alto, base));
  let menuGuardar = $state(false);
  let confirmar = $state(false);
  let guardando = $state(false);
  let error = $state("");
  let cargada = $state(false);

  const cambios = $derived(hayCambios(e));
  const identidad = $derived(esIdentidad(e));
  const puedeOriginal = $derived(cambios);
  const puedeCopia = $derived(!identidad);
  const medida = $derived(tamanoRecorte(e));

  const NOMBRES: Record<Proporcion, string> = {
    libre: "Libre",
    original: "Original",
    cuadrado: "Cuadrado",
    "4:3": "4:3",
    "3:2": "3:2",
    "16:9": "16:9",
  };

  function nombreProporcion(p: Proporcion, vertical: boolean): string {
    if (!vertical || p === "libre" || p === "cuadrado" || p === "original") return NOMBRES[p];
    const [a, b] = p.split(":");
    return `${b}:${a}`;
  }

  // Área de trabajo: la foto visible cabe con un margen, centrada.
  let anchoEscenario = $state(0);
  let altoEscenario = $state(0);
  const MARGEN_X = 64;
  const MARGEN_Y = 40;
  const caja = $derived.by(() => {
    const v = tamanoVisible(e);
    const escala = Math.max(
      0,
      Math.min(
        (anchoEscenario - 2 * MARGEN_X) / v.ancho,
        (altoEscenario - 2 * MARGEN_Y) / v.alto,
      ),
    );
    const w = v.ancho * escala;
    const h = v.alto * escala;
    return {
      escala,
      x: (anchoEscenario - w) / 2,
      y: (altoEscenario - h) / 2,
      w,
      h,
    };
  });

  // La imagen se pinta sin girar y se gira con CSS: primero el giro y luego
  // el volteo, el mismo orden que la receta.
  const imagen = $derived.by(() => {
    const w = e.ancho * caja.escala;
    const h = e.alto * caja.escala;
    const transformacion =
      (e.flip_horizontal ? "scaleX(-1) " : "") + `rotate(${e.rotate}deg)`;
    return {
      x: caja.x + caja.w / 2 - w / 2,
      y: caja.y + caja.h / 2 - h / 2,
      w,
      h,
      transformacion,
    };
  });

  const marco = $derived({
    x: caja.x + e.recorte.x * caja.w,
    y: caja.y + e.recorte.y * caja.h,
    w: e.recorte.width * caja.w,
    h: e.recorte.height * caja.h,
  });

  // Arrastre: se calcula siempre desde el estado del principio, para que los
  // límites no se acumulen. El puntero queda capturado por el tirador y sus
  // movimientos suben hasta el marco.
  type Tirador = "mover" | Esquina | Lado;
  let arrastre = $state<{ tirador: Tirador; x0: number; y0: number; inicio: EstadoEditor } | null>(
    null,
  );

  function empezar(ev: PointerEvent, tirador: Tirador) {
    if (ev.button !== 0) return;
    ev.preventDefault();
    ev.stopPropagation();
    (ev.currentTarget as Element).setPointerCapture(ev.pointerId);
    arrastre = { tirador, x0: ev.clientX, y0: ev.clientY, inicio: e };
  }

  function alMover(ev: PointerEvent) {
    if (!arrastre || caja.w === 0 || caja.h === 0) return;
    const dx = (ev.clientX - arrastre.x0) / caja.w;
    const dy = (ev.clientY - arrastre.y0) / caja.h;
    const { tirador, inicio } = arrastre;
    if (tirador === "mover") e = mover(inicio, dx, dy);
    else if (tirador.length === 2) e = arrastrarEsquina(inicio, tirador as Esquina, dx, dy);
    else if (tirador === "e" || tirador === "o") e = arrastrarLado(inicio, tirador, dx);
    else e = arrastrarLado(inicio, tirador as Lado, dy);
  }

  function soltar() {
    arrastre = null;
  }

  function cancelar() {
    if (guardando) return;
    if (cambios) confirmar = true;
    else onCerrar(null);
  }

  async function guardar(como: "original" | "copia") {
    menuGuardar = false;
    if (guardando) return;
    guardando = true;
    error = "";
    const g = geometria($state.snapshot(e) as EstadoEditor);
    try {
      if (como === "original") {
        await guardarEdicion(id, g);
        onCerrar({ como });
      } else {
        onCerrar({ como, id: await guardarCopia(id, g) });
      }
    } catch (err) {
      // El editor sigue abierto con los cambios para volver a intentarlo.
      const texto = String(err);
      error = texto.charAt(0).toUpperCase() + texto.slice(1);
    } finally {
      guardando = false;
    }
  }

  function alPulsar(ev: KeyboardEvent) {
    if (ev.ctrlKey || ev.metaKey || ev.altKey) return;
    if (confirmar) {
      if (ev.key === "Escape") confirmar = false;
      else return;
    } else if (ev.key === "Escape") {
      if (menuGuardar) menuGuardar = false;
      else cancelar();
    } else {
      const nuevo = tecla(e, ev.key, ev.shiftKey);
      if (!nuevo) return;
      e = nuevo;
    }
    ev.preventDefault();
    ev.stopPropagation();
  }

  let menu = $state<HTMLDivElement>();
  function alPulsarFuera(ev: PointerEvent) {
    if (menuGuardar && menu && !menu.contains(ev.target as Node)) menuGuardar = false;
  }

  const ESQUINAS: Esquina[] = ["no", "ne", "so", "se"];
  const LADOS: Lado[] = ["n", "s", "e", "o"];

  let raiz = $state<HTMLDivElement>();
  $effect(() => raiz?.focus());
</script>

<svelte:window onkeydowncapture={alPulsar} onpointerdown={alPulsarFuera} />

<div
  class="editor"
  role="dialog"
  aria-modal="true"
  aria-label={`Editar ${nombre}`}
  tabindex="-1"
  bind:this={raiz}
>
  <header>
    <button type="button" class="boton" aria-label="Cancelar" title="Cancelar (Esc)" onclick={cancelar}>
      <Icono nombre="cerrar" />
    </button>
    <span class="titulo">Editar</span>
    <span class="nombre">{nombre}</span>
    <span class="hueco"></span>
    <button type="button" class="pildora" disabled={guardando} onclick={cancelar}>Cancelar</button>
    <div class="guardar" bind:this={menu}>
      <button
        type="button"
        class="pildora principal"
        aria-haspopup="menu"
        aria-expanded={menuGuardar}
        disabled={guardando || (!puedeOriginal && !puedeCopia)}
        onclick={() => (menuGuardar = !menuGuardar)}
      >
        {guardando ? "Guardando…" : "Guardar"}
        <Icono nombre="desplegar" tamano={16} grosor={2} />
      </button>
      {#if menuGuardar}
        <div class="menu" role="menu" aria-label="Guardar">
          <button type="button" role="menuitem" disabled={!puedeOriginal} onclick={() => guardar("original")}>
            Guardar como original
            <span>Solo se verá la foto editada. El archivo no se toca: «Volver al original» la recupera.</span>
          </button>
          <button type="button" role="menuitem" disabled={!puedeCopia} onclick={() => guardar("copia")}>
            Guardar como copia
            <span>Crea otra foto junto a la original, con su fecha y sus datos.</span>
          </button>
        </div>
      {/if}
    </div>
  </header>

  <div
    class="escenario"
    bind:clientWidth={anchoEscenario}
    bind:clientHeight={altoEscenario}
  >
    <img
      class="foto"
      class:visible={cargada}
      src={originalUrl(id)}
      alt={nombre}
      draggable="false"
      onload={() => (cargada = true)}
      style:left="{imagen.x}px"
      style:top="{imagen.y}px"
      style:width="{imagen.w}px"
      style:height="{imagen.h}px"
      style:transform={imagen.transformacion}
    />
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="marco"
      class:arrastrando={arrastre !== null}
      style:left="{marco.x}px"
      style:top="{marco.y}px"
      style:width="{marco.w}px"
      style:height="{marco.h}px"
      onpointerdown={(ev) => empezar(ev, "mover")}
      onpointermove={alMover}
      onpointerup={soltar}
      onpointercancel={soltar}
    >
      {#if arrastre}
        <span class="tercio v" style:left="33.333%"></span>
        <span class="tercio v" style:left="66.666%"></span>
        <span class="tercio h" style:top="33.333%"></span>
        <span class="tercio h" style:top="66.666%"></span>
      {/if}
      {#each ESQUINAS as esquina (esquina)}
        <span class="esquina {esquina}" onpointerdown={(ev) => empezar(ev, esquina)}></span>
      {/each}
      {#each LADOS as lado (lado)}
        <span class="lado {lado}" onpointerdown={(ev) => empezar(ev, lado)}></span>
      {/each}
      <span class="medida">
        {medida.ancho} × {medida.alto}{e.proporcion === "libre"
          ? ""
          : ` · ${nombreProporcion(e.proporcion, e.vertical)}`}
      </span>
    </div>

    {#if error}
      <p class="aviso" role="alert">{error}</p>
    {/if}

    {#if confirmar}
      <div class="fondo">
        <div class="dialogo" role="alertdialog" aria-modal="true" aria-labelledby="descartar-titulo">
          <h2 id="descartar-titulo">¿Descartar los cambios?</h2>
          <p>Lo que has cambiado en esta foto no se ha guardado.</p>
          <div class="botones">
            <button type="button" class="pildora" onclick={() => (confirmar = false)}>Seguir editando</button>
            <button type="button" class="pildora principal" onclick={() => onCerrar(null)}>Descartar</button>
          </div>
        </div>
      </div>
    {/if}
  </div>

  <div class="barra" role="toolbar" aria-label="Herramientas">
    <button type="button" class="herramienta" title="Girar a la izquierda (R)" onclick={() => (e = girar(e, "izquierda"))}>
      <Icono nombre="girar" />Girar
    </button>
    <button type="button" class="herramienta" title="Voltear en horizontal" onclick={() => (e = voltear(e))}>
      <Icono nombre="voltear" />Voltear
    </button>
    <span class="separador"></span>
    <div class="chips" role="radiogroup" aria-label="Proporción">
      {#each PROPORCIONES as p (p)}
        <button
          type="button"
          class="chip"
          class:elegida={e.proporcion === p}
          role="radio"
          aria-checked={e.proporcion === p}
          onclick={() => (e = elegirProporcion(e, p))}
        >
          {nombreProporcion(p, e.vertical)}
        </button>
      {/each}
    </div>
    <button
      type="button"
      class="herramienta"
      aria-pressed={e.vertical}
      disabled={e.proporcion === "libre" || e.proporcion === "cuadrado"}
      onclick={() => (e = alternarVertical(e))}
    >
      <Icono nombre="vertical" />Vertical
    </button>
    <span class="separador"></span>
    <button type="button" class="texto" disabled={identidad} onclick={() => (e = restablecer(e))}>
      Restablecer
    </button>
  </div>
</div>

<style>
  .editor {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: grid;
    grid-template-rows: auto 1fr auto;
    background: var(--visor-fondo);
    color: var(--visor-texto);
    outline: none;
    user-select: none;
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 8px 12px;
  }

  .titulo {
    font-weight: 600;
  }

  .nombre {
    overflow: hidden;
    color: var(--visor-texto-suave);
    font-family: var(--fuente-mono);
    font-size: 0.85rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hueco {
    flex: 1;
  }

  .boton {
    display: grid;
    place-items: center;
    width: var(--objetivo);
    height: var(--objetivo);
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--visor-texto);
    cursor: pointer;
  }

  .boton:hover {
    background: var(--visor-superficie);
  }

  .pildora {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    padding: 0 16px;
    border: 1px solid #3a3b40;
    border-radius: 18px;
    background: transparent;
    color: var(--visor-texto);
    cursor: pointer;
  }

  .pildora.principal {
    border-color: #7aa7f5;
    background: #7aa7f5; /* --acento del tema oscuro: el visor siempre es oscuro */
    color: #0d1a33;
    font-weight: 600;
  }

  .pildora:disabled,
  .herramienta:disabled,
  .texto:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .guardar {
    position: relative;
  }

  .menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 3;
    display: grid;
    width: 320px;
    padding: 6px;
    border-radius: var(--radio);
    background: var(--visor-superficie);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.5);
  }

  .menu button {
    display: grid;
    gap: 2px;
    padding: 10px 12px;
    border: 0;
    border-radius: calc(var(--radio) - 2px);
    background: transparent;
    color: var(--visor-texto);
    font: inherit;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }

  .menu button:hover:not(:disabled),
  .menu button:focus-visible {
    background: #232327;
    outline: none;
  }

  .menu button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .menu span {
    color: var(--visor-texto-suave);
    font-size: 0.8rem;
    font-weight: 400;
  }

  .escenario {
    position: relative;
    min-height: 0;
    overflow: hidden;
    touch-action: none;
  }

  .foto {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .foto.visible {
    opacity: 1;
  }

  .marco {
    position: absolute;
    border: 1px solid rgb(255 255 255 / 0.95);
    box-shadow: 0 0 0 100vmax rgb(0 0 0 / 0.62);
    cursor: move;
  }

  .tercio {
    position: absolute;
    background: rgb(255 255 255 / 0.45);
    pointer-events: none;
  }

  .tercio.v {
    top: 0;
    bottom: 0;
    width: 1px;
  }

  .tercio.h {
    left: 0;
    right: 0;
    height: 1px;
  }

  .esquina {
    position: absolute;
    width: 26px;
    height: 26px;
    border: 0 solid #fff;
  }

  .esquina.no {
    left: -5px;
    top: -5px;
    border-left-width: 4px;
    border-top-width: 4px;
    cursor: nwse-resize;
  }

  .esquina.ne {
    right: -5px;
    top: -5px;
    border-right-width: 4px;
    border-top-width: 4px;
    cursor: nesw-resize;
  }

  .esquina.so {
    left: -5px;
    bottom: -5px;
    border-left-width: 4px;
    border-bottom-width: 4px;
    cursor: nesw-resize;
  }

  .esquina.se {
    right: -5px;
    bottom: -5px;
    border-right-width: 4px;
    border-bottom-width: 4px;
    cursor: nwse-resize;
  }

  /* Los lados son franjas anchas para que sea fácil cogerlos. */
  .lado {
    position: absolute;
  }

  .lado::after {
    content: "";
    position: absolute;
    background: #fff;
    border-radius: 2px;
  }

  .lado.n,
  .lado.s {
    left: 26px;
    right: 26px;
    height: 16px;
    cursor: ns-resize;
  }

  .lado.n {
    top: -8px;
  }

  .lado.s {
    bottom: -8px;
  }

  .lado.n::after,
  .lado.s::after {
    left: calc(50% - 14px);
    top: 6px;
    width: 28px;
    height: 4px;
  }

  .lado.e,
  .lado.o {
    top: 26px;
    bottom: 26px;
    width: 16px;
    cursor: ew-resize;
  }

  .lado.e {
    right: -8px;
  }

  .lado.o {
    left: -8px;
  }

  .lado.e::after,
  .lado.o::after {
    top: calc(50% - 14px);
    left: 6px;
    width: 4px;
    height: 28px;
  }

  .medida {
    position: absolute;
    left: 50%;
    bottom: -34px;
    padding: 3px 10px;
    border-radius: 12px;
    background: var(--visor-superficie);
    color: var(--visor-texto-suave);
    font-family: var(--fuente-mono);
    font-size: 0.78rem;
    white-space: nowrap;
    transform: translateX(-50%);
    pointer-events: none;
  }

  .aviso {
    position: absolute;
    top: 12px;
    left: 50%;
    max-width: calc(100% - 160px);
    margin: 0;
    padding: 8px 14px;
    border-radius: var(--radio);
    background: var(--visor-superficie);
    color: var(--visor-error);
    overflow-wrap: anywhere;
    transform: translateX(-50%);
  }

  .fondo {
    position: absolute;
    inset: 0;
    z-index: 5;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 0.55);
  }

  .dialogo {
    width: 380px;
    padding: 22px 22px 16px;
    border-radius: 12px;
    background: var(--visor-superficie);
    box-shadow: 0 16px 40px rgb(0 0 0 / 0.5);
  }

  .dialogo h2 {
    margin: 0 0 6px;
    font-size: 1.05rem;
  }

  .dialogo p {
    margin: 0 0 18px;
    color: var(--visor-texto-suave);
    font-size: 0.9rem;
  }

  .botones {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .barra {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 14px 16px 18px;
  }

  .separador {
    width: 1px;
    height: 28px;
    margin: 0 6px;
    background: #2c2d31;
  }

  .herramienta {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 14px;
    border: 0;
    border-radius: 20px;
    background: var(--visor-superficie);
    color: var(--visor-texto);
    font-size: 0.9rem;
    cursor: pointer;
  }

  .herramienta[aria-pressed="true"] {
    background: #ececee;
    color: #0d0d0e;
  }

  .chips {
    display: flex;
    gap: 4px;
    padding: 4px;
    border-radius: 22px;
    background: var(--visor-superficie);
  }

  .chip {
    height: 32px;
    padding: 0 12px;
    border: 0;
    border-radius: 16px;
    background: transparent;
    color: var(--visor-texto-suave);
    font-size: 0.85rem;
    cursor: pointer;
  }

  .chip.elegida {
    background: #ececee;
    color: #0d0d0e;
    font-weight: 600;
  }

  .texto {
    padding: 4px 0;
    border: 0;
    background: none;
    color: var(--visor-texto);
    font-size: 0.85rem;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }
</style>
