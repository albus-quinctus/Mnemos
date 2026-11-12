<script lang="ts">
  import { irA, navegacion, type Seccion } from "$lib/navegacion.svelte";
  import Icono, { type NombreIcono } from "./Icono.svelte";

  type Entrada = { nombre: string; icono: NombreIcono; seccion?: Seccion };

  // Las entradas sin `seccion` llegan en fases posteriores.
  const entradas: Entrada[] = [
    { nombre: "Fotos", icono: "fotos", seccion: "fotos" },
    { nombre: "Explorar", icono: "explorar" },
    { nombre: "Álbumes", icono: "albumes" },
    { nombre: "Etiquetas", icono: "etiquetas" },
    { nombre: "Favoritos", icono: "estrella", seccion: "favoritos" },
    { nombre: "Archivo", icono: "archivo", seccion: "archivo" },
    { nombre: "Papelera", icono: "papelera" },
  ];

  // Utilidades, debajo de una línea (spec 010).
  const utilidades: Entrada[] = [
    { nombre: "Duplicados", icono: "duplicados", seccion: "duplicados" },
  ];
</script>

{#snippet lista(items: Entrada[])}
  <ul>
    {#each items as entrada (entrada.nombre)}
      {@const actual = entrada.seccion !== undefined && navegacion.seccion === entrada.seccion}
      <li>
        <button
          type="button"
          class:actual
          aria-current={actual ? "page" : undefined}
          disabled={!entrada.seccion}
          title={entrada.seccion ? undefined : "Próximamente"}
          onclick={() => entrada.seccion && irA(entrada.seccion)}
        >
          <Icono nombre={entrada.icono} />
          <span>{entrada.nombre}</span>
        </button>
      </li>
    {/each}
  </ul>
{/snippet}

<nav aria-label="Secciones">
  <div class="marca">MNEMOS</div>
  {@render lista(entradas)}
  <hr />
  {@render lista(utilidades)}
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

  hr {
    width: calc(100% - 1.5rem);
    margin: 0.25rem auto;
    border: 0;
    border-top: 1px solid var(--borde);
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
