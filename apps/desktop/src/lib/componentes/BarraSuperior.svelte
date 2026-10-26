<script lang="ts">
  import Icono from "./Icono.svelte";
</script>

<header>
  <label class="buscar">
    <Icono nombre="buscar" />
    <!-- La búsqueda queda fuera de la spec 002. -->
    <input type="search" placeholder="Buscar en tus fotos (próximamente)" disabled />
  </label>
  <button type="button" class="importar" disabled title="Próximamente">
    <Icono nombre="importar" />
    <span>Importar</span>
  </button>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 1rem;
    height: var(--barra-alto);
    padding: 0 1.5rem;
    border-bottom: 1px solid var(--borde);
    background: var(--fondo);
  }

  .buscar {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 0.6rem;
    max-width: 40rem;
    min-height: var(--objetivo);
    padding: 0 0.9rem;
    border-radius: var(--radio);
    background: var(--superficie-alta);
    color: var(--texto-suave);
  }

  input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    outline: none;
  }

  input::placeholder {
    color: var(--texto-suave);
  }

  .importar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-height: var(--objetivo);
    margin-left: auto;
    padding: 0 1rem;
    border: 1px solid var(--borde);
    border-radius: var(--radio);
    background: var(--superficie);
    cursor: pointer;
  }

  .importar:disabled {
    color: var(--texto-suave);
    cursor: default;
  }
</style>
