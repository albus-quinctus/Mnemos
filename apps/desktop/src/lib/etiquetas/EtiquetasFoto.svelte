<script lang="ts">
  // Sección «Etiquetas» del panel Detalles del visor (spec 016): una pastilla
  // por etiqueta (al pulsarla se abre la etiqueta) con su cruz para quitarla,
  // y «Añadir etiqueta», con las mismas sugerencias que la barra de selección
  // y la opción de crear. Solo cambia la base de datos, nunca el archivo.
  import { tick } from "svelte";
  import {
    crearEtiqueta,
    etiquetasDeFoto,
    ponerEtiqueta,
    quitarEtiqueta,
    type Etiqueta,
  } from "$lib/biblioteca";
  import { avisarCambio } from "$lib/cambios.svelte";
  import Icono from "$lib/componentes/Icono.svelte";
  import {
    existeNombre,
    filtrarEtiquetas,
    MAX_NOMBRE,
    nombreValido,
    ordenarEtiquetas,
    textoFotos,
  } from "./etiquetas";
  import { cargarEtiquetas, estadoEtiquetas } from "./estado.svelte";

  let {
    foto,
    pedido = false,
    onAtendido,
    onAbrir,
  }: {
    foto: number;
    /** La tecla `T` del visor pide abrir el campo y enfocarlo. */
    pedido?: boolean;
    onAtendido?: () => void;
    onAbrir: (etiqueta: number) => void;
  } = $props();

  let lista = $state<Etiqueta[]>([]);
  let anadiendo = $state(false);
  let texto = $state("");
  let error = $state("");
  let ocupado = $state(false);
  let campo = $state<HTMLInputElement>();

  const puestas = $derived(new Set(lista.map((e) => e.id)));
  const sugerencias = $derived(
    filtrarEtiquetas(estadoEtiquetas.lista, texto)
      .filter((e) => !puestas.has(e.id))
      .slice(0, 6),
  );
  const nuevo = $derived(nombreValido(texto));
  const puedeCrear = $derived(nuevo !== null && !existeNombre(estadoEtiquetas.lista, nuevo));

  // Al abrir y al cambiar de foto: sus etiquetas, y el campo cerrado si
  // venía de otra foto (al abrir el panel con `T` ya está abierto).
  let anterior: number | null = null;
  $effect(() => {
    const id = foto;
    if (anterior !== null && anterior !== id) {
      anadiendo = false;
      texto = "";
    }
    anterior = id;
    error = "";
    cargar(id);
  });

  async function cargar(id: number) {
    try {
      const suyas = ordenarEtiquetas(await etiquetasDeFoto(id));
      if (id === foto) lista = suyas;
    } catch (e) {
      if (id === foto) error = String(e);
    }
  }

  // La tecla `T` puede llegar antes de que el panel exista: el pedido se
  // atiende al montar la sección.
  $effect(() => {
    if (pedido) {
      onAtendido?.();
      enfocar();
    }
  });

  /** Abre el campo y lo enfoca (botón «Añadir etiqueta» y tecla `T`). */
  async function enfocar() {
    anadiendo = true;
    texto = "";
    error = "";
    cargarEtiquetas();
    await tick();
    campo?.focus();
  }

  async function poner(etiqueta: Etiqueta) {
    if (ocupado) return;
    ocupado = true;
    const id = foto;
    try {
      await ponerEtiqueta([id], etiqueta.id);
      texto = "";
      await cargar(id);
      avisarCambio();
      cargarEtiquetas();
    } catch (e) {
      error = String(e);
    } finally {
      ocupado = false;
    }
  }

  async function crearYPoner() {
    if (!nuevo || ocupado) return;
    try {
      await poner(await crearEtiqueta(nuevo));
    } catch (e) {
      error = String(e);
    }
  }

  async function quitar(etiqueta: Etiqueta) {
    if (ocupado) return;
    ocupado = true;
    const id = foto;
    try {
      await quitarEtiqueta([id], etiqueta.id);
      await cargar(id);
      avisarCambio();
      cargarEtiquetas();
    } catch (e) {
      error = String(e);
    } finally {
      ocupado = false;
    }
  }

  function alPulsar(e: KeyboardEvent) {
    if (e.key === "Escape") {
      // Esc cierra el campo, no el visor.
      e.preventDefault();
      e.stopPropagation();
      anadiendo = false;
    } else if (e.key === "Enter") {
      e.preventDefault();
      // La primera fila: la primera sugerencia o, si no hay, «Crear».
      if (sugerencias.length > 0) poner(sugerencias[0]);
      else if (puedeCrear) crearYPoner();
    }
  }

  function alSalir(e: FocusEvent) {
    // Cerrar al salir del bloque, salvo si el foco va a una sugerencia.
    const a = e.relatedTarget;
    if (a instanceof HTMLElement && a.closest(".anadir")) return;
    if (texto.trim() === "") anadiendo = false;
  }
</script>

<section class="etiquetas-foto" aria-label="Etiquetas">
  <h3>Etiquetas</h3>
  {#if lista.length > 0}
    <ul class="pastillas">
      {#each lista as etiqueta (etiqueta.id)}
        <li class="pastilla">
          <button
            type="button"
            class="nombre"
            title={`Abrir «${etiqueta.name}»`}
            onclick={() => onAbrir(etiqueta.id)}
          >
            {etiqueta.name}
          </button>
          <button
            type="button"
            class="cruz"
            aria-label={`Quitar «${etiqueta.name}»`}
            title="Quitar"
            disabled={ocupado}
            onclick={() => quitar(etiqueta)}
          >
            <Icono nombre="cerrar" tamano={14} grosor={2.2} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if anadiendo}
    <div class="anadir">
      <input
        bind:this={campo}
        bind:value={texto}
        class="campo-etiqueta"
        maxlength={MAX_NOMBRE}
        placeholder="Buscar o crear etiqueta"
        aria-label="Buscar o crear etiqueta"
        onkeydown={alPulsar}
        onblur={alSalir}
      />
      {#if sugerencias.length > 0 || puedeCrear}
        <ul class="sugerencias" role="listbox" aria-label="Sugerencias">
          {#each sugerencias as etiqueta, i (etiqueta.id)}
            <li>
              <button
                type="button"
                class:primera={i === 0 && texto.trim() !== ""}
                disabled={ocupado}
                onclick={() => poner(etiqueta)}
              >
                <span class="texto-sugerencia">{etiqueta.name}</span>
                <span class="numero">{textoFotos(etiqueta.photos)}</span>
              </button>
            </li>
          {/each}
          {#if puedeCrear}
            <li>
              <button type="button" class="crear" disabled={ocupado} onclick={crearYPoner}>
                <Icono nombre="mas" tamano={16} grosor={2.2} />
                Crear «{nuevo}»
              </button>
            </li>
          {/if}
        </ul>
      {/if}
    </div>
  {:else}
    <button type="button" class="pastilla anadir-boton" title="Añadir etiqueta (T)" onclick={enfocar}>
      <Icono nombre="mas" tamano={14} grosor={2.2} />
      Añadir etiqueta
    </button>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
  /* El visor siempre es oscuro: colores --visor-* y el acento del tema oscuro. */
  h3 {
    margin: 28px 0 8px;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .pastillas {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 0 0 8px;
    padding: 0;
    list-style: none;
  }

  .pastilla {
    display: inline-flex;
    align-items: center;
    max-width: 100%;
    height: 32px;
    border-radius: 999px;
    background: rgb(255 255 255 / 0.1);
    color: var(--visor-texto);
  }

  .pastilla button {
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  .nombre {
    min-width: 0;
    height: 100%;
    padding: 0 6px 0 14px;
    overflow: hidden;
    border-radius: 999px 0 0 999px;
    font-size: 0.9rem;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .nombre:hover {
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .cruz {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 28px;
    height: 28px;
    margin-right: 2px;
    border-radius: 999px;
    color: var(--visor-texto-suave) !important;
  }

  .cruz:hover {
    background: rgb(255 255 255 / 0.12) !important;
    color: var(--visor-texto) !important;
  }

  .anadir-boton {
    gap: 6px;
    padding: 0 14px 0 10px;
    border: 1px dashed rgb(255 255 255 / 0.3);
    background: none;
    color: var(--visor-texto-suave);
    font: inherit;
    font-size: 0.9rem;
    cursor: pointer;
  }

  .anadir-boton:hover {
    border-color: rgb(255 255 255 / 0.5);
    color: var(--visor-texto);
  }

  .campo-etiqueta {
    box-sizing: border-box;
    width: 100%;
    height: 40px;
    padding: 0 12px;
    border: 2px solid #7aa7f5;
    border-radius: 8px;
    background: var(--visor-fondo);
    color: var(--visor-texto);
    font: inherit;
    outline: none;
  }

  .sugerencias {
    margin: 8px 0 0;
    padding: 4px;
    border-radius: 10px;
    background: rgb(255 255 255 / 0.05);
    list-style: none;
  }

  .sugerencias button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 40px;
    padding: 0 12px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--visor-texto);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .sugerencias button:hover,
  .sugerencias button:focus-visible,
  .sugerencias .primera {
    background: rgb(255 255 255 / 0.08);
    outline: none;
  }

  .texto-sugerencia {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .numero {
    color: var(--visor-texto-suave);
    font-size: 0.85rem;
  }

  .sugerencias .crear {
    color: #7aa7f5;
    font-weight: 600;
  }

  .error {
    margin: 6px 0 0;
    color: var(--visor-error);
    font-size: 0.85rem;
  }
</style>
