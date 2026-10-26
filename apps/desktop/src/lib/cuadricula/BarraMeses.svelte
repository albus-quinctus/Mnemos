<script lang="ts">
  // Barra de la derecha para saltar en el tiempo, como en Google Fotos: los
  // años marcados, un punto por mes y el mes bajo el puntero al pasar por ella.
  import { mesEn, nombreMes, type Marca } from "./disposicion";

  let {
    marcas,
    alto,
    altoVista,
    desplazamiento,
    onIr,
  }: {
    marcas: Marca[];
    alto: number;
    altoVista: number;
    desplazamiento: number;
    onIr: (y: number) => void;
  } = $props();

  let barra = $state<HTMLDivElement>();
  let altoBarra = $state(0);
  let puntero = $state<number | null>(null);
  let arrastrando = false;

  // Posición en la barra (0 a 1) de un punto de la cuadrícula, y al revés.
  const recorrido = $derived(Math.max(1, alto - altoVista));
  const fraccion = (y: number) => Math.min(1, Math.max(0, y / recorrido));

  const anios = $derived.by(() => {
    const vistos = new Set<string>();
    return marcas.filter((m) => {
      const anio = m.mes.slice(0, 4);
      if (vistos.has(anio)) return false;
      vistos.add(anio);
      return true;
    });
  });

  const actual = $derived(fraccion(desplazamiento));
  const mesPuntero = $derived(
    puntero === null ? undefined : mesEn(marcas, (puntero / Math.max(1, altoBarra)) * recorrido),
  );

  function posicion(e: PointerEvent): number {
    const rect = barra!.getBoundingClientRect();
    return Math.min(rect.height, Math.max(0, e.clientY - rect.top));
  }

  function ir(e: PointerEvent) {
    const p = posicion(e);
    puntero = p;
    onIr((p / Math.max(1, altoBarra)) * recorrido);
  }
</script>

<div
  class="barra"
  bind:this={barra}
  bind:clientHeight={altoBarra}
  role="scrollbar"
  aria-label="Saltar a un mes"
  aria-controls="cuadricula"
  aria-orientation="vertical"
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={Math.round(actual * 100)}
  tabindex="-1"
  onpointerdown={(e) => {
    arrastrando = true;
    barra?.setPointerCapture(e.pointerId);
    ir(e);
  }}
  onpointermove={(e) => (arrastrando ? ir(e) : (puntero = posicion(e)))}
  onpointerup={() => (arrastrando = false)}
  onpointerleave={() => {
    if (!arrastrando) puntero = null;
  }}
>
  {#each marcas as marca (marca.mes)}
    <span class="punto" style:top="{fraccion(marca.y) * 100}%"></span>
  {/each}
  {#each anios as marca (marca.mes)}
    <span class="anio" style:top="{fraccion(marca.y) * 100}%">{marca.mes.slice(0, 4)}</span>
  {/each}
  <span class="actual" style:top="{actual * 100}%"></span>
  {#if puntero !== null && mesPuntero}
    <span class="burbuja" style:top="{puntero}px">{nombreMes(mesPuntero)}</span>
  {/if}
</div>

<style>
  .barra {
    position: absolute;
    top: 12px;
    right: 0;
    bottom: 12px;
    width: 56px;
    cursor: pointer;
    touch-action: none;
    user-select: none;
  }

  .punto,
  .anio,
  .actual,
  .burbuja {
    position: absolute;
    transform: translateY(-50%);
  }

  .punto {
    right: 6px;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--borde);
  }

  .anio {
    right: 18px;
    padding: 0 4px;
    border-radius: 4px;
    background: var(--fondo);
    color: var(--texto-suave);
    font-size: 0.75rem;
    font-weight: 500;
  }

  .actual {
    right: 0;
    width: 16px;
    height: 2px;
    background: var(--acento);
  }

  .burbuja {
    right: 56px;
    padding: 6px 10px;
    border-radius: var(--radio);
    background: var(--texto);
    color: var(--fondo);
    font-size: 0.85rem;
    white-space: nowrap;
    pointer-events: none;
  }
</style>
