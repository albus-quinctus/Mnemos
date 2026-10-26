<script lang="ts">
  import "$lib/estilos/tokens.css";
  import BarraSuperior from "$lib/componentes/BarraSuperior.svelte";
  import MenuLateral from "$lib/componentes/MenuLateral.svelte";

  let { children } = $props();
</script>

<div class="galeria">
  <MenuLateral />
  <div class="columna">
    <BarraSuperior />
    <main>
      {@render children()}
    </main>
  </div>
</div>

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
