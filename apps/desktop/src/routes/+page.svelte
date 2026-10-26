<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type LibraryInfo = {
    root: string;
    schema_version: number;
    photos: number;
    albums: number;
    tags: number;
  };

  let ruta = $state("");
  let info = $state<LibraryInfo | null>(null);
  let error = $state("");
  let cargando = $state(false);

  async function abrir(event: Event) {
    event.preventDefault();
    cargando = true;
    error = "";
    info = null;
    try {
      info = await invoke<LibraryInfo>("abrir_biblioteca", { ruta });
    } catch (e) {
      error = String(e);
    } finally {
      cargando = false;
    }
  }
</script>

<section class="abrir">
  <h1>Fotos</h1>
  <p class="lema">Tu biblioteca de fotos, portátil y privada.</p>

  <form onsubmit={abrir}>
    <label for="ruta">Carpeta de la biblioteca</label>
    <div class="fila">
      <input id="ruta" placeholder="/media/disco/Biblioteca" bind:value={ruta} required />
      <button type="submit" disabled={cargando}>Abrir</button>
    </div>
  </form>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if info}
    <dl>
      <dt>Biblioteca</dt><dd>{info.root}</dd>
      <dt>Esquema</dt><dd>v{info.schema_version}</dd>
      <dt>Fotos</dt><dd>{info.photos}</dd>
      <dt>Álbumes</dt><dd>{info.albums}</dd>
      <dt>Etiquetas</dt><dd>{info.tags}</dd>
    </dl>
  {/if}
</section>

<style>
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

  form {
    margin-top: 2rem;
  }

  label {
    display: block;
    font-size: 0.9rem;
    margin-bottom: 0.4rem;
  }

  .fila {
    display: flex;
    gap: 0.5rem;
  }

  input {
    flex: 1;
    min-height: var(--objetivo);
    padding: 0 0.8rem;
    border: 1px solid var(--borde);
    border-radius: var(--radio);
    background: var(--superficie);
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

  button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .error {
    color: var(--error);
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.4rem 1.5rem;
    margin-top: 2rem;
  }

  dt {
    color: var(--texto-suave);
  }

  dd {
    margin: 0;
    font-family: var(--fuente-mono);
    font-size: 0.9em;
    word-break: break-all;
  }
</style>
