<script lang="ts">
  // Cabecera de una pila abierta (spec 023): volver, «Pila», cuántas fotos y
  // de dónde salió, y «Desapilar». Dentro de la pila cada acción afecta solo a
  // las fotos que se eligen.
  import { desapilar, deshacerPila, fotosDePila, type FotosDePila } from "$lib/biblioteca";
  import Icono from "$lib/componentes/Icono.svelte";
  import { cerrarPila } from "$lib/navegacion.svelte";
  import { avisarCambio } from "$lib/cambios.svelte";
  import { mostrarAviso } from "$lib/seleccion/acciones.svelte";
  import { nombreTipo, textoFotos } from "./pilas.svelte";

  let { id, version = 0 }: { id: number; version?: number } = $props();

  let pila = $state<FotosDePila | null>(null);

  $effect(() => {
    void version;
    fotosDePila(id)
      .then((p) => (pila = p))
      .catch(() => (pila = null));
  });

  const subtitulo = $derived(
    pila ? `${textoFotos(pila.photos.length)} · ${nombreTipo(pila.kind)}` : "",
  );

  async function separar() {
    const n = pila?.photos.length ?? 0;
    try {
      const antes = await desapilar([id]);
      avisarCambio();
      cerrarPila();
      mostrarAviso({
        texto: `Pila deshecha: ${textoFotos(n)} sueltas`,
        deshacer: async () => {
          await deshacerPila(antes);
          avisarCambio();
        },
      });
    } catch (e) {
      mostrarAviso({ texto: `No se ha podido desapilar: ${String(e)}`, error: true });
    }
  }
</script>

<header class="cabecera">
  <button type="button" class="volver" onclick={cerrarPila}>
    <Icono nombre="anterior" tamano={18} />
    <span>Volver</span>
  </button>
  <div class="fila">
    <div>
      <h1>Pila</h1>
      <p class="subtitulo">{subtitulo}&nbsp;</p>
    </div>
    <button type="button" class="desapilar" onclick={separar}>
      <Icono nombre="desapilar" />
      <span>Desapilar</span>
    </button>
  </div>
  <p class="nota">Aquí cada acción afecta solo a las fotos que elijas.</p>
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

  .volver:hover,
  .desapilar:hover {
    background: var(--superficie-alta);
  }

  .fila {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 1rem;
  }

  h1 {
    margin: 0.4rem 0 0;
    font-size: 2rem;
    font-weight: 650;
    line-height: 1.3;
  }

  .subtitulo {
    margin: 0.15rem 0 0;
    color: var(--texto-suave);
  }

  .desapilar {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    height: var(--objetivo);
    padding: 0 1rem;
    border: 1px solid var(--borde);
    border-radius: 999px;
    background: var(--superficie);
    color: var(--texto);
    font-weight: 500;
    cursor: pointer;
  }

  .nota {
    margin: 0.75rem 0 0;
    color: var(--texto-suave);
    font-size: 0.9rem;
  }
</style>
