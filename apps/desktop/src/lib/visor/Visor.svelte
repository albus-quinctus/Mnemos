<script lang="ts">
  // Visor a pantalla completa, estilo Cuarto oscuro (decisión 003). Muestra al
  // instante la miniatura ampliada y la cambia por la foto completa en cuanto
  // carga. La anterior y la siguiente se precargan en memoria.
  import { onDestroy } from "svelte";
  import type { FotoCuadricula } from "$lib/biblioteca";
  import Icono from "$lib/componentes/Icono.svelte";
  import { fotoUrl, miniaturaUrl } from "$lib/media";
  import { CacheImagenes, vecinas } from "./cache";

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

  onDestroy(() => cache.vaciar());

  function ir(paso: number) {
    const destino = indice + paso;
    if (destino >= 0 && destino < fotos.length) onCambiar(destino);
  }

  function alPulsar(e: KeyboardEvent) {
    if (e.key === "Escape") onCerrar();
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

<svelte:window onkeydown={alPulsar} />

<div
  class="visor"
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
</div>

<style>
  .visor {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: grid;
    grid-template-rows: auto 1fr auto;
    background: var(--visor-fondo);
    color: var(--visor-texto);
    outline: none;
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
