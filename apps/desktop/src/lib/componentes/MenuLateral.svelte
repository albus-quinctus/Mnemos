<script lang="ts">
  import Icono, { type NombreIcono } from "./Icono.svelte";

  // Solo "Fotos" existe por ahora; el resto llega en fases posteriores.
  const entradas: { nombre: string; icono: NombreIcono; activa: boolean }[] = [
    { nombre: "Fotos", icono: "fotos", activa: true },
    { nombre: "Explorar", icono: "explorar", activa: false },
    { nombre: "Álbumes", icono: "albumes", activa: false },
    { nombre: "Etiquetas", icono: "etiquetas", activa: false },
    { nombre: "Favoritos", icono: "favoritos", activa: false },
    { nombre: "Papelera", icono: "papelera", activa: false },
  ];
</script>

<nav aria-label="Secciones">
  <div class="marca">MNEMOS</div>
  <ul>
    {#each entradas as entrada (entrada.nombre)}
      <li>
        <button
          type="button"
          class:actual={entrada.activa}
          aria-current={entrada.activa ? "page" : undefined}
          disabled={!entrada.activa}
          title={entrada.activa ? undefined : "Próximamente"}
        >
          <Icono nombre={entrada.icono} />
          <span>{entrada.nombre}</span>
        </button>
      </li>
    {/each}
  </ul>
</nav>

<style>
  nav {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 1rem 0.75rem;
    border-right: 1px solid var(--borde);
    background: var(--superficie);
  }

  .marca {
    display: flex;
    align-items: center;
    height: var(--objetivo);
    padding: 0 0.75rem;
    font-weight: 700;
    letter-spacing: 0.2em;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  button {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    min-height: var(--objetivo);
    padding: 0 0.75rem;
    border: 0;
    border-radius: var(--radio);
    background: transparent;
    color: var(--texto);
    text-align: left;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    background: var(--superficie-alta);
  }

  .actual {
    background: var(--acento-suave);
    color: var(--acento);
    font-weight: 600;
  }

  button:disabled {
    color: var(--texto-suave);
    cursor: default;
  }
</style>
