<script lang="ts" module>
  // Última carpeta a la que se exportó en esta sesión: el selector la propone.
  let ultimaCarpeta: string | undefined;
</script>

<script lang="ts">
  // Visor a pantalla completa, estilo Cuarto oscuro (decisión 003). Muestra al
  // instante la miniatura ampliada y la cambia por la foto completa en cuanto
  // carga. La anterior y la siguiente se precargan en memoria.
  import { open } from "@tauri-apps/plugin-dialog";
  import { onDestroy } from "svelte";
  import {
    camara as pedirCamara,
    detalles as pedirDetalles,
    exportar,
    ponerFecha,
    quitarFechaManual,
    type Camara,
    type Detalles,
    type FotoCuadricula,
    type TamanoExportacion,
  } from "$lib/biblioteca";
  import Icono from "$lib/componentes/Icono.svelte";
  import { fotoUrl, miniaturaUrl } from "$lib/media";
  import { CacheImagenes, vecinas } from "./cache";
  import { fechas } from "./fechas.svelte";
  import {
    apertura,
    camposFecha,
    coordenadas,
    dimensiones,
    exposicion,
    fecha,
    fechaCaptura,
    fechaManual,
    focal,
    tamano,
  } from "./formato";

  let {
    fotos,
    indice,
    quedan,
    onCambiar,
    onCerrar,
    onPedirMas,
  }: {
    fotos: FotoCuadricula[];
    indice: number;
    quedan: boolean;
    onCambiar: (indice: number) => void;
    onCerrar: () => void;
    onPedirMas: () => void;
  } = $props();

  type Completa = {
    img: HTMLImageElement;
    estado: "cargando" | "lista" | "error";
    aviso?: string;
  };

  // Sube cada vez que una imagen termina de cargar, para volver a pintar.
  let cambios = $state(0);

  const cache = new CacheImagenes<Completa>(
    5,
    (id) => {
      const img = new Image();
      const completa: Completa = { img, estado: "cargando" };
      img.decoding = "async";
      img.onload = () => {
        completa.estado = "lista";
        cambios++;
      };
      img.onerror = async () => {
        completa.estado = "error";
        completa.aviso = await motivo(id);
        cambios++;
      };
      img.src = fotoUrl(id);
      return completa;
    },
    (completa) => {
      // Soltar la imagen para que el motor web libere la memoria.
      completa.img.onload = completa.img.onerror = null;
      completa.img.src = "";
    },
  );

  /** Por qué no se pudo mostrar: el protocolo responde 404 si falta el archivo. */
  async function motivo(id: number): Promise<string> {
    try {
      const respuesta = await fetch(fotoUrl(id));
      if (respuesta.status === 404) return "El archivo no está en el disco";
    } catch {
      // Sin respuesta: se da el aviso general.
    }
    return "No se puede mostrar esta foto";
  }

  const foto = $derived(fotos[indice]);
  // Copia del estado de la imagen: si fuera el mismo objeto, Svelte no vería
  // que ha cambiado al terminar de cargar.
  const completa = $derived.by(() => {
    void cambios;
    if (!foto) return undefined;
    const { img, estado, aviso } = cache.obtener(foto.id);
    return { src: img.src, estado, aviso };
  });

  // Precarga de las vecinas y, cerca del final de lo cargado, más páginas.
  $effect(() => {
    cache.precargar(vecinas(indice, fotos.length).map((i) => fotos[i].id));
    if (quedan && indice >= fotos.length - 3) onPedirMas();
  });

  onDestroy(() => {
    cache.vaciar();
    // Las cuadrículas se recargan al cerrar, no antes: así la foto no se
    // mueve de la tira mientras se mira.
    if (fechaCambiada) fechas.version++;
  });

  function ir(paso: number) {
    const destino = indice + paso;
    if (destino >= 0 && destino < fotos.length) onCambiar(destino);
  }

  // Panel de detalles: se abre y se cierra con la tecla i y sigue abierto al
  // pasar de foto.
  let panel = $state(false);
  let info = $state<Detalles | null>(null);
  let camaraInfo = $state<Camara | null>(null);
  // Sube al guardar una fecha, para volver a pedir los detalles.
  let recarga = $state(0);
  const captura = $derived(info ? fechaCaptura(info.taken_at, info.taken_source) : null);
  const automatica = $derived(
    info?.taken_source === "manual"
      ? fechaCaptura(info.taken_auto_at, info.taken_auto_source)
      : null,
  );
  $effect(() => {
    if (!panel || !foto) return;
    const id = foto.id;
    void recarga;
    let vigente = true;
    pedirDetalles(id)
      .then((d) => {
        if (vigente) info = d;
      })
      .catch(() => {
        if (vigente) info = null;
      });
    // El EXIF se lee del original: si el disco no está, solo falta la sección.
    pedirCamara(id)
      .then((c) => {
        if (vigente) camaraInfo = c;
      })
      .catch(() => {
        if (vigente) camaraInfo = null;
      });
    return () => {
      vigente = false;
    };
  });

  // Fecha a mano (spec 011). Solo cambia la base de datos.
  let editando = $state(false);
  let dia = $state("");
  let hora = $state("");
  let guardando = $state(false);
  let errorFecha = $state("");
  let fechaCambiada = false;

  // Al pasar de foto se cierra el formulario sin guardar.
  $effect(() => {
    void foto?.id;
    editando = false;
    errorFecha = "";
  });

  function empezarEdicion() {
    if (!info) return;
    ({ dia, hora } = camposFecha(info.taken_at ?? info.modified_at));
    errorFecha = "";
    editando = true;
  }

  async function cambiarFecha(accion: () => Promise<boolean>) {
    guardando = true;
    errorFecha = "";
    try {
      if (!(await accion())) {
        errorFecha = "Esta foto ya no está en la biblioteca.";
        return;
      }
      fechaCambiada = true;
      editando = false;
      recarga++;
    } catch (e) {
      errorFecha = String(e);
    } finally {
      guardando = false;
    }
  }

  function guardarFecha(e: SubmitEvent) {
    e.preventDefault();
    if (!foto) return;
    const valor = fechaManual(dia, hora);
    if (!valor) {
      errorFecha = "Escribe el día y la hora.";
      return;
    }
    const id = foto.id;
    cambiarFecha(() => ponerFecha(id, valor));
  }

  function volverAAutomatica() {
    if (!foto) return;
    const id = foto.id;
    cambiarFecha(() => quitarFechaManual(id));
  }

  // Exportar (spec 008): menú con los dos tamaños, selector de carpeta y un
  // aviso con el resultado.
  let menuExportar = $state(false);
  let exportando = $state(false);
  let mensaje = $state<{ texto: string; error: boolean } | null>(null);
  let menu = $state<HTMLDivElement>();
  let temporizador: ReturnType<typeof setTimeout> | undefined;

  function avisar(texto: string, error: boolean) {
    clearTimeout(temporizador);
    mensaje = { texto, error };
    temporizador = setTimeout(() => (mensaje = null), error ? 8000 : 5000);
  }

  async function exportarComo(tamano: TamanoExportacion) {
    menuExportar = false;
    if (!foto || exportando) return;
    const id = foto.id;
    const carpeta = await open({
      directory: true,
      multiple: false,
      title: "Elige dónde guardar la foto",
      defaultPath: ultimaCarpeta,
    });
    if (typeof carpeta !== "string") return;
    ultimaCarpeta = carpeta;
    exportando = true;
    try {
      avisar(`Guardada en ${await exportar(id, carpeta, tamano)}`, false);
    } catch (e) {
      const texto = String(e);
      avisar(texto.charAt(0).toUpperCase() + texto.slice(1), true);
    } finally {
      exportando = false;
    }
  }

  function alPulsarFuera(e: PointerEvent) {
    if (menuExportar && menu && !menu.contains(e.target as Node)) menuExportar = false;
  }

  onDestroy(() => clearTimeout(temporizador));

  function alPulsar(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    // Mientras se escribe la fecha, las teclas son del formulario.
    if (editando && e.target instanceof HTMLElement && e.target.closest(".editar-fecha")) {
      if (e.key === "Escape") {
        editando = false;
        e.preventDefault();
      }
      return;
    }
    if (e.key === "Escape" && menuExportar) menuExportar = false;
    else if (e.key === "Escape") onCerrar();
    else if (e.key === "i" || e.key === "I") panel = !panel;
    else if (e.key === "ArrowRight") ir(1);
    else if (e.key === "ArrowLeft") ir(-1);
    else return;
    e.preventDefault();
  }

  // Tira de miniaturas: unas pocas a cada lado de la actual.
  const TIRA = 15;
  const tira = $derived(
    fotos
      .slice(Math.max(0, indice - TIRA), indice + TIRA + 1)
      .map((f, n) => ({ foto: f, indice: Math.max(0, indice - TIRA) + n })),
  );

  let actualEnTira = $state<HTMLButtonElement>();
  $effect(() => {
    actualEnTira?.scrollIntoView({ inline: "center", block: "nearest" });
  });

  let visor = $state<HTMLDivElement>();
  $effect(() => visor?.focus());
</script>

<svelte:window onkeydown={alPulsar} onpointerdown={alPulsarFuera} />

<div
  class="visor"
  class:con-panel={panel}
  role="dialog"
  aria-modal="true"
  aria-label={foto?.name ?? "Visor"}
  tabindex="-1"
  bind:this={visor}
>
  <header>
    <button type="button" class="boton" aria-label="Cerrar" title="Cerrar (Esc)" onclick={onCerrar}>
      <Icono nombre="cerrar" />
    </button>
    <span class="nombre">{foto?.name}</span>
    <div class="exportar" bind:this={menu}>
      <button
        type="button"
        class="boton"
        class:activo={menuExportar}
        aria-label="Exportar"
        aria-haspopup="menu"
        aria-expanded={menuExportar}
        title="Exportar"
        disabled={exportando}
        onclick={() => (menuExportar = !menuExportar)}
      >
        <Icono nombre="exportar" />
      </button>
      {#if menuExportar}
        <div class="menu" role="menu" aria-label="Exportar">
          <button type="button" role="menuitem" onclick={() => exportarComo("original")}>
            Tamaño original
            <span>El archivo tal cual, con sus datos</span>
          </button>
          <button type="button" role="menuitem" onclick={() => exportarComo("reducida")}>
            Tamaño reducido (2048 px)
            <span>JPEG para compartir, sin ubicación</span>
          </button>
        </div>
      {/if}
    </div>
    <button
      type="button"
      class="boton info"
      class:activo={panel}
      aria-label="Detalles"
      aria-pressed={panel}
      title="Detalles (i)"
      onclick={() => (panel = !panel)}
    >
      <Icono nombre="info" />
    </button>
  </header>

  <div class="escenario">
    {#if foto}
      {#key foto.id}
        <!-- La miniatura ampliada se ve al instante; la completa la tapa al cargar. -->
        <img class="miniatura" src={miniaturaUrl(foto.id)} alt="" draggable="false" />
        {#if completa?.estado === "lista"}
          <img class="completa" src={completa.src} alt={foto.name} draggable="false" />
        {/if}
      {/key}
      {#if completa?.estado === "error"}
        <p class="aviso" role="status">{completa.aviso}</p>
      {/if}
    {/if}
    {#if exportando}
      <p class="aviso mensaje" role="status">Exportando…</p>
    {:else if mensaje}
      <p class="aviso mensaje" class:error={mensaje.error} role={mensaje.error ? "alert" : "status"}>
        {mensaje.texto}
      </p>
    {/if}

    {#if indice > 0}
      <button type="button" class="boton flecha izquierda" aria-label="Anterior" onclick={() => ir(-1)}>
        <Icono nombre="anterior" tamano={28} />
      </button>
    {/if}
    {#if indice < fotos.length - 1}
      <button type="button" class="boton flecha derecha" aria-label="Siguiente" onclick={() => ir(1)}>
        <Icono nombre="siguiente" tamano={28} />
      </button>
    {/if}
  </div>

  <nav class="tira" aria-label="Fotos cercanas">
    {#each tira as { foto: f, indice: i } (f.id)}
      {#if i === indice}
        <button type="button" class="pequena actual" aria-current="true" bind:this={actualEnTira}>
          <img src={miniaturaUrl(f.id)} alt={f.name} draggable="false" />
        </button>
      {:else}
        <button type="button" class="pequena" onclick={() => onCambiar(i)}>
          <img src={miniaturaUrl(f.id)} alt={f.name} draggable="false" />
        </button>
      {/if}
    {/each}
  </nav>

  {#if panel}
    <aside class="panel" aria-label="Detalles">
      <h2>Detalles</h2>
      {#if info}
        <dl>
          <dt>Ruta</dt>
          <dd class="mono">{info.rel_path}</dd>
          <dt>Dimensiones</dt>
          <dd>{dimensiones(info.width, info.height)}</dd>
          <dt>Tamaño</dt>
          <dd>{tamano(info.size_bytes)}</dd>
          <dt>Fecha</dt>
          <dd>
            {#if editando}
              <form class="editar-fecha" onsubmit={guardarFecha}>
                <label>
                  <span>Día</span>
                  <input type="date" min="1900-01-01" max="2200-12-31" bind:value={dia} required />
                </label>
                <label>
                  <span>Hora</span>
                  <input type="time" step="1" bind:value={hora} required />
                </label>
                <div class="acciones-fecha">
                  <button type="submit" class="texto principal" disabled={guardando}>Guardar</button>
                  <button type="button" class="texto" onclick={() => (editando = false)}>
                    Cancelar
                  </button>
                </div>
              </form>
            {:else}
              {captura?.texto}
              {#if captura?.origen}<span class="origen">{captura?.origen}</span>{/if}
              <div class="acciones-fecha">
                <button type="button" class="texto" onclick={empezarEdicion}>Cambiar fecha</button>
                {#if info.taken_source === "manual"}
                  <button
                    type="button"
                    class="texto"
                    disabled={guardando}
                    title={automatica
                      ? `${automatica.texto}${automatica.origen ? ` · ${automatica.origen}` : ""}`
                      : undefined}
                    onclick={volverAAutomatica}
                  >
                    Volver a la fecha automática
                  </button>
                {/if}
              </div>
            {/if}
            {#if errorFecha}<span class="error-fecha" role="alert">{errorFecha}</span>{/if}
          </dd>
          <dt>Modificada</dt>
          <dd>{fecha(info.modified_at)}</dd>
          <dt>Formato</dt>
          <dd>{info.format || "Desconocido"}</dd>
        </dl>
        {#if camaraInfo}
          {@const c = camaraInfo}
          {@const distancia = focal(c.focal_mm, c.focal_35mm)}
          <h3>Cámara</h3>
          <dl>
            {#if c.camera}<dt>Cámara</dt><dd>{c.camera}</dd>{/if}
            {#if c.lens}<dt>Objetivo</dt><dd>{c.lens}</dd>{/if}
            {#if c.exposure_s !== null || c.f_number !== null || c.iso !== null}
              <dt>Exposición</dt>
              <dd>
                {[
                  c.exposure_s !== null ? exposicion(c.exposure_s) : null,
                  c.f_number !== null ? apertura(c.f_number) : null,
                  c.iso !== null ? `ISO ${c.iso}` : null,
                ]
                  .filter(Boolean)
                  .join(" · ")}
              </dd>
            {/if}
            {#if distancia}<dt>Distancia focal</dt><dd>{distancia}</dd>{/if}
            {#if c.flash !== null}<dt>Flash</dt><dd>{c.flash ? "Saltó" : "No saltó"}</dd>{/if}
            {#if c.gps}<dt>Ubicación</dt><dd class="mono">{coordenadas(c.gps)}</dd>{/if}
          </dl>
        {/if}
        <p class="nota">El original no se modifica nunca.</p>
      {/if}
    </aside>
  {/if}
</div>

<style>
  .visor {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: grid;
    grid-template-rows: auto 1fr auto;
    grid-template-columns: minmax(0, 1fr);
    background: var(--visor-fondo);
    color: var(--visor-texto);
    outline: none;
  }

  .visor.con-panel {
    grid-template-columns: minmax(0, 1fr) 320px;
  }

  .panel {
    grid-column: 2;
    grid-row: 1 / -1;
    padding: 20px;
    border-left: 1px solid var(--visor-superficie);
    background: var(--visor-fondo);
    overflow-y: auto;
  }

  .panel h2 {
    margin: 4px 0 20px;
    font-size: 1rem;
    font-weight: 600;
  }

  dl {
    display: grid;
    gap: 4px;
    margin: 0;
  }

  dt {
    margin-top: 12px;
    color: var(--visor-texto-suave);
    font-size: 0.8rem;
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .panel h3 {
    margin: 28px 0 4px;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .acciones-fecha {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-top: 6px;
  }

  .texto {
    padding: 4px 0;
    border: 0;
    background: none;
    color: var(--visor-texto);
    font: inherit;
    font-size: 0.85rem;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }

  .texto.principal {
    padding: 4px 12px;
    border-radius: var(--radio);
    background: var(--visor-superficie);
    text-decoration: none;
  }

  .texto:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .editar-fecha {
    display: grid;
    gap: 8px;
    margin-top: 4px;
  }

  .editar-fecha label {
    display: grid;
    gap: 2px;
    font-size: 0.8rem;
    color: var(--visor-texto-suave);
  }

  .editar-fecha input {
    padding: 6px 8px;
    border: 1px solid var(--visor-texto-suave);
    border-radius: var(--radio);
    background: transparent;
    color: var(--visor-texto);
    font: inherit;
    color-scheme: dark;
  }

  .error-fecha {
    display: block;
    margin-top: 6px;
    color: #f2938c; /* --error del tema oscuro: el visor siempre es oscuro */
    font-size: 0.8rem;
  }

  .mono {
    font-family: var(--fuente-mono);
    font-size: 0.85rem;
  }

  .origen {
    display: block;
    color: var(--visor-texto-suave);
    font-size: 0.8rem;
  }

  .nota {
    margin-top: 24px;
    color: var(--visor-texto-suave);
    font-size: 0.8rem;
  }

  .exportar {
    position: relative;
    margin-left: auto;
  }

  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 1;
    display: grid;
    min-width: 260px;
    padding: 6px;
    border-radius: var(--radio);
    background: var(--visor-superficie);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.4);
  }

  .menu button {
    display: grid;
    gap: 2px;
    padding: 8px 12px;
    border: 0;
    border-radius: calc(var(--radio) - 2px);
    background: transparent;
    color: var(--visor-texto);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .menu button:hover,
  .menu button:focus-visible {
    background: var(--visor-fondo);
    outline: none;
  }

  .menu span {
    color: var(--visor-texto-suave);
    font-size: 0.8rem;
  }

  .boton:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .boton.activo {
    background: var(--visor-superficie);
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 8px 12px;
  }

  .nombre {
    overflow: hidden;
    color: var(--visor-texto-suave);
    font-family: var(--fuente-mono);
    font-size: 0.85rem;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .escenario {
    position: relative;
    min-height: 0;
    overflow: hidden;
  }

  .escenario img {
    position: absolute;
    inset: 16px 72px;
    width: calc(100% - 144px);
    height: calc(100% - 32px);
    object-fit: contain;
    user-select: none;
  }

  .aviso {
    position: absolute;
    left: 50%;
    bottom: 24px;
    margin: 0;
    padding: 8px 14px;
    border-radius: var(--radio);
    background: var(--visor-superficie);
    transform: translateX(-50%);
  }

  .mensaje {
    top: 16px;
    bottom: auto;
    max-width: calc(100% - 160px);
    overflow-wrap: anywhere;
  }

  .mensaje.error {
    color: var(--visor-error);
  }

  .flecha {
    position: absolute;
    top: 50%;
    width: 56px;
    height: 56px;
    transform: translateY(-50%);
  }

  .izquierda {
    left: 8px;
  }

  .derecha {
    right: 8px;
  }

  .tira {
    display: flex;
    justify-content: safe center;
    gap: 4px;
    padding: 8px 12px 12px;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .pequena {
    flex: none;
    width: 64px;
    height: 48px;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 4px;
    background: var(--visor-superficie);
    opacity: 0.6;
    cursor: pointer;
    overflow: hidden;
  }

  .pequena:hover,
  .pequena.actual {
    opacity: 1;
  }

  .pequena.actual {
    border-color: var(--visor-texto);
  }

  .pequena img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
