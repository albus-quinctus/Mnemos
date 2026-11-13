<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount, untrack } from "svelte";
  import { busqueda, limpiarBusqueda } from "$lib/busqueda/busqueda.svelte";
  import Cuadricula from "$lib/cuadricula/Cuadricula.svelte";
  import { olvidarTitulos } from "$lib/cuadricula/titulos.svelte";
  import Duplicados from "$lib/duplicados/Duplicados.svelte";
  import { navegacion } from "$lib/navegacion.svelte";
  import { cambiarAmbito } from "$lib/seleccion/seleccion.svelte";
  import { importacion } from "$lib/importacion/importacion.svelte";
  import { vaciarCaducadas } from "$lib/papelera/papelera.svelte";

  type LibraryInfo = {
    root: string;
    schema_version: number;
    photos: number;
    albums: number;
    tags: number;
  };

  let info = $state<LibraryInfo | null>(null);
  let error = $state("");
  let ocupado = $state(false);
  // Mientras se intenta abrir la última biblioteca no se ofrece nada más.
  let arrancando = $state(true);

  async function elegirCarpeta(titulo: string): Promise<string | null> {
    const ruta = await open({ directory: true, multiple: false, title: titulo });
    return typeof ruta === "string" ? ruta : null;
  }

  async function usar(comando: "abrir_biblioteca" | "crear_biblioteca", ruta: string) {
    ocupado = true;
    error = "";
    try {
      info = await invoke<LibraryInfo>(comando, { ruta });
      // Lo que lleva 30 días en la Papelera se elimina al abrir y cada hora.
      vaciarCaducadas();
    } catch (e) {
      error = String(e);
    } finally {
      ocupado = false;
    }
  }

  async function abrir() {
    const ruta = await elegirCarpeta("Elige la carpeta de la biblioteca");
    if (ruta) await usar("abrir_biblioteca", ruta);
  }

  async function crear() {
    const ruta = await elegirCarpeta("Elige la carpeta para la nueva biblioteca");
    if (ruta) await usar("crear_biblioteca", ruta);
  }

  // Fotos queda debajo, sin desmontarse, mientras se ve otra cosa encima.
  const tapada = $derived(busqueda.consulta !== "" || navegacion.seccion !== "fotos");

  // La selección es de la vista o de la búsqueda que se ve: al cambiar de
  // sección, buscar o salir de la búsqueda, se pierde (spec 006).
  $effect(() => {
    const seccion = navegacion.seccion;
    const consulta = busqueda.consulta;
    untrack(() => {
      if (consulta) cambiarAmbito({ busqueda: consulta });
      else cambiarAmbito({ vista: seccion === "duplicados" ? "fotos" : seccion });
    });
  });

  // Cada biblioteca empieza sin búsqueda.
  $effect(() => {
    busqueda.disponible = info !== null;
    void info?.root;
    limpiarBusqueda();
    olvidarTitulos();
  });

  onMount(async () => {
    try {
      const ultima = await invoke<string | null>("ultima_biblioteca");
      if (ultima) await usar("abrir_biblioteca", ultima);
    } catch (e) {
      error = String(e);
    } finally {
      arrancando = false;
    }
  });
</script>

{#if info}
  <div class="capas">
    <!-- Se monta de nuevo con cada biblioteca para empezar desde cero. Durante
         una búsqueda queda oculta debajo, para volver al mismo punto. -->
    {#key `${info.root}#${importacion.version}`}
      <div class="capa" class:oculta={tapada}>
        <Cuadricula activa={!tapada} />
      </div>
    {/key}
    <!-- Favoritos, Archivo y Papelera empiezan de cero cada vez que se abren. -->
    {#if navegacion.seccion === "favoritos" || navegacion.seccion === "archivo" || navegacion.seccion === "papelera"}
      {#key `${info.root}#${importacion.version}#${navegacion.seccion}`}
        <div class="capa encima" class:oculta={busqueda.consulta !== ""}>
          <Cuadricula vista={navegacion.seccion} activa={!busqueda.consulta} />
        </div>
      {/key}
    {/if}
    {#if navegacion.seccion === "duplicados" && !busqueda.consulta}
      {#key `${info.root}#${importacion.version}`}
        <div class="capa encima">
          <Duplicados />
        </div>
      {/key}
    {/if}
    {#if busqueda.consulta}
      {#key `${busqueda.consulta}#${importacion.version}`}
        <div class="capa resultados">
          <Cuadricula texto={busqueda.consulta} onSalir={limpiarBusqueda} />
        </div>
      {/key}
    {/if}
  </div>
{:else}
  <section class="abrir">
    <h1>Fotos</h1>
    <p class="lema">Tu biblioteca de fotos, portátil y privada.</p>

    {#if arrancando}
      <p class="aviso">Abriendo tu última biblioteca…</p>
    {:else}
      <div class="acciones">
        <button type="button" onclick={abrir} disabled={ocupado}>Abrir biblioteca…</button>
        <button type="button" class="secundario" onclick={crear} disabled={ocupado}>
          Crear biblioteca…
        </button>
      </div>
      <p class="ayuda">
        Una biblioteca es una carpeta con tus fotos. Al crearla, MNEMOS añade sus datos
        dentro de ella y no toca los archivos que ya tenga.
      </p>
    {/if}

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
  </section>
{/if}

<style>
  .capas {
    position: relative;
    height: 100%;
  }

  .capa {
    position: absolute;
    inset: 0;
  }

  .oculta {
    visibility: hidden;
  }

  .resultados,
  .encima {
    background: var(--fondo);
  }

  .encima {
    overflow: auto;
  }

  .abrir {
    max-width: 40rem;
    padding: 2rem 1.5rem;
  }

  h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
  }

  .lema {
    color: var(--texto-suave);
    margin-top: 0.25rem;
  }

  .acciones {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin-top: 2rem;
  }

  button {
    min-height: var(--objetivo);
    padding: 0 1.2rem;
    border: 0;
    border-radius: var(--radio);
    background: var(--acento);
    color: var(--acento-texto);
    font-weight: 600;
    cursor: pointer;
  }

  button.secundario {
    border: 1px solid var(--borde);
    background: var(--superficie);
    color: var(--texto);
  }

  button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .ayuda,
  .aviso {
    margin-top: 1.25rem;
    color: var(--texto-suave);
    font-size: 0.9rem;
  }

  .error {
    color: var(--error);
  }
</style>
