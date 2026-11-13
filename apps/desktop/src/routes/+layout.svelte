<script lang="ts">
  import "$lib/estilos/tokens.css";
  import BarraSuperior from "$lib/componentes/BarraSuperior.svelte";
  import MenuLateral from "$lib/componentes/MenuLateral.svelte";
  import { escucharImportacion } from "$lib/importacion/importacion.svelte";
  import ProgresoImportacion from "$lib/importacion/ProgresoImportacion.svelte";
  import Ajustes from "$lib/ajustes/Ajustes.svelte";
  import { ajustes } from "$lib/ajustes/ajustes.svelte";
  import { esTeclaPantallaCompleta, pantallaCompleta } from "$lib/ajustes/pantalla";
  import Aviso from "$lib/seleccion/Aviso.svelte";
  import BarraSeleccion from "$lib/seleccion/BarraSeleccion.svelte";
  import { haySeleccion } from "$lib/seleccion/seleccion.svelte";
  import { onMount } from "svelte";

  let { children } = $props();

  onMount(escucharImportacion);

  // F11 alterna la pantalla completa en cualquier pantalla (spec 013): en la
  // fase de captura, para que valga también en el visor, el editor y los campos
  // de texto, y sin escribir nada en ellos.
  function alPulsar(e: KeyboardEvent) {
    if (!esTeclaPantallaCompleta(e)) return;
    e.preventDefault();
    // Con Ajustes abierta, el interruptor sigue a la tecla.
    pantallaCompleta().then((estado) => {
      if (estado !== null) ajustes.pantallaCompleta = estado;
    });
  }
</script>

<svelte:window onkeydowncapture={alPulsar} />

<div class="galeria">
  <MenuLateral />
  <div class="columna">
    {#if haySeleccion()}
      <BarraSeleccion />
    {:else}
      <BarraSuperior />
    {/if}
    <main>
      {@render children()}
    </main>
  </div>
</div>

<ProgresoImportacion />
<Ajustes />
<Aviso />

<style>
  .galeria {
    display: grid;
    grid-template-columns: var(--menu-ancho) 1fr;
    height: 100vh;
  }

  .columna {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  main {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
</style>
