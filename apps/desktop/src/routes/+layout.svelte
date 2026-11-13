<script lang="ts">
  import "$lib/estilos/tokens.css";
  import BarraSuperior from "$lib/componentes/BarraSuperior.svelte";
  import MenuLateral from "$lib/componentes/MenuLateral.svelte";
  import { escucharExportacion } from "$lib/exportacion/exportacion.svelte";
  import ProgresoExportacion from "$lib/exportacion/ProgresoExportacion.svelte";
  import { escucharImportacion } from "$lib/importacion/importacion.svelte";
  import ProgresoImportacion from "$lib/importacion/ProgresoImportacion.svelte";
  import Aviso from "$lib/seleccion/Aviso.svelte";
  import BarraSeleccion from "$lib/seleccion/BarraSeleccion.svelte";
  import { haySeleccion } from "$lib/seleccion/seleccion.svelte";
  import { onMount } from "svelte";

  let { children } = $props();

  onMount(() => {
    escucharImportacion();
    escucharExportacion();
  });
</script>

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
<ProgresoExportacion />
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
