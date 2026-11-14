<script lang="ts">
  // «Etiquetar» de la barra de selección (spec 016): un campo para buscar o
  // crear y la lista de etiquetas con una casilla marcada si todas las fotos
  // seleccionadas la tienen, con guion si solo algunas. Cada clic se guarda al
  // momento; al cerrar sale el aviso con «Deshacer».
  import { tick } from "svelte";
  import { crearEtiqueta, ponerEtiqueta, quitarEtiqueta, usoEtiquetas } from "$lib/biblioteca";
  import Icono from "$lib/componentes/Icono.svelte";
  import { avisos, mostrarAviso } from "$lib/seleccion/acciones.svelte";
  import { seleccion } from "$lib/seleccion/seleccion.svelte";
  import { terminarEtiquetado } from "./acciones.svelte";
  import {
    casilla,
    existeNombre,
    filtrarEtiquetas,
    MAX_NOMBRE,
    nombreValido,
    type CambioEtiqueta,
  } from "./etiquetas";
  import { cargarEtiquetas, estadoEtiquetas } from "./estado.svelte";

  let abierto = $state(false);
  let texto = $state("");
  let ocupado = $state(false);
  /** Cuántas fotos seleccionadas tiene cada etiqueta. */
  let uso = $state(new Map<number, number>());
  let cambios: (CambioEtiqueta & { id: number })[] = [];
  let caja = $state<HTMLDivElement>();
  let campo = $state<HTMLInputElement>();

  const total = $derived(seleccion.ids.size);
  const visibles = $derived(filtrarEtiquetas(estadoEtiquetas.lista, texto));
  const nuevo = $derived(nombreValido(texto));
  const puedeCrear = $derived(nuevo !== null && !existeNombre(estadoEtiquetas.lista, nuevo));

  async function abrir() {
    abierto = true;
    texto = "";
    cambios = [];
    const ids = [...seleccion.ids];
    const [, usos] = await Promise.all([cargarEtiquetas(), usoEtiquetas(ids).catch(() => [])]);
    uso = new Map(usos.map((u) => [u.tag, u.photos]));
    await tick();
    campo?.focus();
  }

  function cerrar() {
    if (!abierto) return;
    abierto = false;
    terminarEtiquetado(cambios);
    cambios = [];
  }

  async function alternarEtiqueta(id: number, nombre: string) {
    if (ocupado) return;
    ocupado = true;
    const ids = [...seleccion.ids];
    const poner = casilla(uso.get(id) ?? 0, total) !== "si";
    try {
      const fotos = poner ? await ponerEtiqueta(ids, id) : await quitarEtiqueta(ids, id);
      cambios.push({ id, nombre, puesta: poner, fotos });
      uso = new Map(uso).set(id, poner ? total : 0);
    } catch (e) {
      mostrarAviso({ texto: `No se ha podido completar: ${String(e)}`, error: true });
    } finally {
      ocupado = false;
    }
  }

  async function crear() {
    if (!nuevo || ocupado) return;
    ocupado = true;
    try {
      const etiqueta = await crearEtiqueta(nuevo);
      const fotos = await ponerEtiqueta([...seleccion.ids], etiqueta.id);
      cambios.push({ id: etiqueta.id, nombre: etiqueta.name, puesta: true, fotos });
      uso = new Map(uso).set(etiqueta.id, total);
      texto = "";
      await cargarEtiquetas();
    } catch (e) {
      mostrarAviso({ texto: `No se ha podido completar: ${String(e)}`, error: true });
    } finally {
      ocupado = false;
    }
  }

  // Intro elige la primera fila: la primera etiqueta que queda o «Crear».
  function alPulsarCampo(e: KeyboardEvent) {
    if (e.key !== "Enter") return;
    e.preventDefault();
    if (texto.trim() && visibles.length > 0) alternarEtiqueta(visibles[0].id, visibles[0].name);
    else if (puedeCrear) crear();
  }

  function alPulsarFuera(e: PointerEvent) {
    if (abierto && caja && !caja.contains(e.target as Node)) cerrar();
  }

  function alPulsarTecla(e: KeyboardEvent) {
    if (e.key === "Escape" && abierto) {
      e.preventDefault();
      e.stopPropagation();
      cerrar();
    }
  }
</script>

<svelte:window onpointerdown={alPulsarFuera} onkeydowncapture={alPulsarTecla} />

<div class="etiquetar" bind:this={caja}>
  <button
    type="button"
    class="boton"
    class:activo={abierto}
    aria-haspopup="dialog"
    aria-expanded={abierto}
    title="Etiquetar"
    disabled={avisos.ocupado}
    onclick={() => (abierto ? cerrar() : abrir())}
  >
    <Icono nombre="etiquetas" />
    <span>Etiquetar</span>
  </button>
  {#if abierto}
    <div class="menu" role="dialog" aria-label="Etiquetar">
      <label class="campo">
        <Icono nombre="buscar" tamano={18} />
        <input
          bind:this={campo}
          bind:value={texto}
          maxlength={MAX_NOMBRE}
          placeholder="Buscar o crear etiqueta"
          aria-label="Buscar o crear etiqueta"
          onkeydown={alPulsarCampo}
        />
      </label>
      <ul class="lista" role="listbox" aria-multiselectable="true" aria-label="Etiquetas">
        {#each visibles as etiqueta, i (etiqueta.id)}
          {@const estado = casilla(uso.get(etiqueta.id) ?? 0, total)}
          <li
            role="option"
            aria-selected={estado === "si"}
            aria-checked={estado === "algo" ? "mixed" : estado === "si"}
            class:primera={i === 0 && texto.trim() !== ""}
          >
            <button
              type="button"
              disabled={ocupado}
              onclick={() => alternarEtiqueta(etiqueta.id, etiqueta.name)}
            >
              <span class="caja {estado}">
                {#if estado === "si"}<Icono nombre="marca" tamano={14} grosor={3} />{/if}
                {#if estado === "algo"}<Icono nombre="menos" tamano={14} grosor={3} />{/if}
              </span>
              <span class="nombre">{etiqueta.name}</span>
              <span class="numero">{etiqueta.photos.toLocaleString("es")}</span>
            </button>
          </li>
        {/each}
        {#if puedeCrear}
          {#if visibles.length > 0}<li class="separador" role="presentation"></li>{/if}
          <li role="option" aria-selected="false" class:primera={visibles.length === 0}>
            <button type="button" class="crear" disabled={ocupado} onclick={crear}>
              <Icono nombre="mas" tamano={18} grosor={2.2} />
              <span class="nombre">Crear «{nuevo}»</span>
            </button>
          </li>
        {:else if estadoEtiquetas.lista.length === 0}
          <li class="nada">Escribe un nombre para crear la primera etiqueta.</li>
        {/if}
      </ul>
    </div>
  {/if}
</div>

<style>
  .etiquetar {
    position: relative;
  }

  .boton {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    height: var(--objetivo);
    padding: 0 0.9rem;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--texto);
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
  }

  .boton:hover:not(:disabled),
  .boton.activo {
    background: var(--superficie-alta);
  }

  .boton:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 5;
    display: flex;
    flex-direction: column;
    width: 20rem;
    max-height: min(30rem, calc(100vh - 6rem));
    border: 1px solid var(--borde);
    border-radius: 14px;
    background: var(--superficie);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.18);
    overflow: hidden;
  }

  .campo {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0.6rem;
    padding: 0 0.75rem;
    border: 2px solid var(--acento);
    border-radius: 10px;
    color: var(--texto-suave);
  }

  .campo input {
    flex: 1;
    min-width: 0;
    height: 36px;
    border: 0;
    background: transparent;
    color: var(--texto);
    outline: none;
  }

  .lista {
    margin: 0;
    padding: 0 0.35rem 0.35rem;
    overflow-y: auto;
    list-style: none;
  }

  .lista button {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    height: 42px;
    padding: 0 0.65rem;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--texto);
    text-align: left;
    cursor: pointer;
  }

  .lista button:hover,
  .lista button:focus-visible,
  .primera button {
    background: var(--superficie-alta);
    outline: none;
  }

  .nombre {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .numero {
    color: var(--texto-suave);
    font-size: 0.85rem;
  }

  .caja {
    display: grid;
    flex: none;
    place-items: center;
    width: 20px;
    height: 20px;
    border: 2px solid var(--texto-suave);
    border-radius: 5px;
  }

  .caja.si,
  .caja.algo {
    border-color: var(--acento);
    background: var(--acento);
    color: var(--acento-texto);
  }

  .crear {
    color: var(--acento) !important;
    font-weight: 600;
  }

  .separador {
    height: 1px;
    margin: 0.25rem 0.6rem;
    background: var(--borde);
  }

  .nada {
    padding: 0.5rem 0.75rem 0.75rem;
    color: var(--texto-suave);
    font-size: 0.9rem;
  }
</style>
