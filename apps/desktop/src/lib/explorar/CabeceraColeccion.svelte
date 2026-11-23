<script lang="ts">
  // Cabecera de una colección o un recuerdo abierto desde Explorar (spec 021):
  // volver a Explorar, el nombre y «N elementos». No se puede renombrar ni
  // borrar.
  import { vistaMeses } from "$lib/biblioteca";
  import Icono from "$lib/componentes/Icono.svelte";
  import { irA, type AbiertoEnExplorar } from "$lib/navegacion.svelte";
  import { textoElementos } from "./explorar";

  let { abierto, version = 0 }: { abierto: AbiertoEnExplorar; version?: number } = $props();

  let total = $state<number | null>(null);

  // Al abrir y cada vez que cambian datos de fotos (`version`).
  $effect(() => {
    void version;
    const vista = abierto.vista;
    vistaMeses(vista)
      .then((meses) => (total = meses.reduce((suma, m) => suma + m.photos, 0)))
      .catch(() => (total = null));
  });

  const subtitulo = $derived(
    [total === null ? null : textoElementos(total), abierto.detalle ?? null]
      .filter((t) => t !== null)
      .join(" · "),
  );
</script>

<header class="cabecera">
  <button type="button" class="volver" onclick={() => irA("explorar")}>
    <Icono nombre="anterior" tamano={18} />
    <span>Explorar</span>
  </button>
  <h1>{abierto.titulo}</h1>
  <p class="subtitulo">{subtitulo}&nbsp;</p>
</header>

<style>
  .cabecera {
    padding: 1.25rem 4.5rem 0.25rem 2rem;
  }

  .volver {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    height: 32px;
    margin-left: -0.5rem;
    padding: 0 0.6rem 0 0.4rem;
    border: 0;
    border-radius: 999px;
    background: none;
    color: var(--texto-suave);
    font-weight: 500;
    cursor: pointer;
  }

  .volver:hover {
    background: var(--superficie-alta);
  }

  h1 {
    margin: 0.4rem 0 0;
    overflow: hidden;
    font-size: 2rem;
    font-weight: 650;
    line-height: 1.3;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .subtitulo {
    margin: 0.15rem 0 0;
    color: var(--texto-suave);
  }
</style>
