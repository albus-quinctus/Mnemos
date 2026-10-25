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

<main>
  <h1>MNEMOS</h1>
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
</main>

<style>
  :global(:root) {
    --fondo: #f6f6f4;
    --texto: #1c1c1a;
    --suave: #6b6b66;
    --borde: #d6d6d0;
    --acento: #2f5d50;
    --error: #a3271f;
    font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
    color: var(--texto);
    background: var(--fondo);
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --fondo: #161615;
      --texto: #ececea;
      --suave: #9b9b95;
      --borde: #3a3a37;
      --acento: #7fbfa9;
      --error: #f08a80;
    }
  }

  main {
    max-width: 40rem;
    margin: 0 auto;
    padding: 3rem 1.5rem;
  }

  h1 {
    margin: 0;
    letter-spacing: 0.2em;
  }

  .lema {
    color: var(--suave);
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
    padding: 0.6rem 0.8rem;
    border: 1px solid var(--borde);
    border-radius: 6px;
    background: transparent;
    color: inherit;
    font: inherit;
  }

  button {
    padding: 0.6rem 1.2rem;
    border: 0;
    border-radius: 6px;
    background: var(--acento);
    color: var(--fondo);
    font: inherit;
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
    color: var(--suave);
  }

  dd {
    margin: 0;
    word-break: break-all;
  }
</style>
