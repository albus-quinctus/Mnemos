<script lang="ts">
  // Ventana de Ajustes (spec 013): modal sobre la galería, cambios al momento.
  import Icono from "../componentes/Icono.svelte";
  import type { Tema } from "../biblioteca";
  import {
    ajustes,
    cambiarPantallaCompleta,
    cambiarTema,
    cerrarAjustes,
  } from "./estado.svelte";
  import { version } from "../../../package.json";

  const temas: { valor: Tema; nombre: string }[] = [
    { valor: "sistema", nombre: "Sistema" },
    { valor: "claro", nombre: "Claro" },
    { valor: "oscuro", nombre: "Oscuro" },
  ];

  // Esc cierra solo Ajustes: ni la búsqueda ni la selección de debajo.
  function alPulsar(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopImmediatePropagation();
      cerrarAjustes();
    }
  }

  // Al abrir, el foco entra en la ventana para que el teclado vaya a ella.
  function enfocar(nodo: HTMLElement) {
    nodo.focus();
  }
</script>

<svelte:window onkeydowncapture={(e) => ajustes.abierta && alPulsar(e)} />

{#if ajustes.abierta}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="fondo" onclick={(e) => e.target === e.currentTarget && cerrarAjustes()}>
    <div
      class="ventana"
      role="dialog"
      aria-modal="true"
      aria-labelledby="ajustes-titulo"
      tabindex="-1"
      use:enfocar
    >
      <header>
        <h2 id="ajustes-titulo">Ajustes</h2>
        <button type="button" class="cerrar" aria-label="Cerrar" onclick={cerrarAjustes}>
          <Icono nombre="cerrar" />
        </button>
      </header>

      <section>
        <h3>Apariencia</h3>
        <div class="fila">
          <div class="texto">
            <span>Tema</span>
            <span class="pista">«Sistema» sigue el modo claro u oscuro del equipo</span>
          </div>
          <div class="segmentado" role="radiogroup" aria-label="Tema">
            {#each temas as t (t.valor)}
              <button
                type="button"
                role="radio"
                aria-checked={ajustes.tema === t.valor}
                class:activo={ajustes.tema === t.valor}
                onclick={() => cambiarTema(t.valor)}
              >
                {t.nombre}
              </button>
            {/each}
          </div>
        </div>
      </section>

      <section>
        <h3>Ventana</h3>
        <div class="fila">
          <div class="texto">
            <span>Pantalla completa</span>
            <span class="pista">También con F11</span>
          </div>
          <button
            type="button"
            role="switch"
            class="interruptor"
            aria-label="Pantalla completa"
            aria-checked={ajustes.pantallaCompleta}
            onclick={cambiarPantallaCompleta}
          >
            <span class="bola"></span>
          </button>
        </div>
      </section>

      <footer>MNEMOS {version}</footer>
    </div>
  </div>
{/if}

<style>
  .fondo {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: rgba(0, 0, 0, 0.4);
  }

  .ventana {
    outline: none;
    width: min(32rem, 100%);
    max-height: calc(100vh - 2rem);
    overflow: auto;
    padding: 0.5rem 1.5rem 1rem;
    border: 1px solid var(--borde);
    border-radius: 12px;
    background: var(--superficie);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: var(--barra-alto);
  }

  h2 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .cerrar {
    display: grid;
    place-items: center;
    width: var(--objetivo);
    height: var(--objetivo);
    margin-right: -0.75rem;
    border: 0;
    border-radius: 50%;
    background: transparent;
    cursor: pointer;
  }

  .cerrar:hover {
    background: var(--superficie-alta);
  }

  section {
    padding: 0.75rem 0;
    border-top: 1px solid var(--borde);
  }

  h3 {
    margin: 0 0 0.5rem;
    color: var(--texto-suave);
    font-size: 0.8rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .fila {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    min-height: var(--objetivo);
  }

  .texto {
    display: flex;
    flex-direction: column;
  }

  .pista {
    color: var(--texto-suave);
    font-size: 0.85rem;
  }

  .segmentado {
    display: flex;
    flex: none;
    padding: 3px;
    border-radius: 999px;
    background: var(--superficie-alta);
  }

  .segmentado button {
    min-height: 34px;
    padding: 0 0.9rem;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--texto-suave);
    cursor: pointer;
  }

  .segmentado button.activo {
    background: var(--superficie);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.18);
    color: var(--texto);
    font-weight: 600;
  }

  .interruptor {
    position: relative;
    flex: none;
    width: 44px;
    height: 26px;
    border: 0;
    border-radius: 999px;
    background: var(--borde);
    cursor: pointer;
    transition: background 0.15s;
  }

  .interruptor[aria-checked="true"] {
    background: var(--acento);
  }

  .bola {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
    transition: transform 0.15s;
  }

  .interruptor[aria-checked="true"] .bola {
    transform: translateX(18px);
  }

  footer {
    padding-top: 0.75rem;
    border-top: 1px solid var(--borde);
    color: var(--texto-suave);
    font-size: 0.8rem;
  }
</style>
