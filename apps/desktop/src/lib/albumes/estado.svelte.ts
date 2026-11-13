// Estado de la pantalla Álbumes (spec 014): la lista y el diálogo «Nuevo
// álbum». Los álbumes solo viven en la base de datos.
import { albumes as pedirAlbumes, crearAlbum, type Album } from "$lib/biblioteca";
import { abrirAlbum } from "$lib/navegacion.svelte";

export const estadoAlbumes = $state({
  lista: [] as Album[],
  cargado: false,
  error: "",
  /** El diálogo «Nuevo álbum» está abierto. */
  creando: false,
});

/** Vuelve a pedir los álbumes (al abrir la pantalla y tras cada cambio). */
export async function cargarAlbumes() {
  try {
    estadoAlbumes.lista = await pedirAlbumes();
    estadoAlbumes.error = "";
  } catch (e) {
    estadoAlbumes.error = String(e);
  } finally {
    estadoAlbumes.cargado = true;
  }
}

export function pedirNuevoAlbum() {
  estadoAlbumes.creando = true;
}

export function cancelarNuevoAlbum() {
  estadoAlbumes.creando = false;
}

/** Crea el álbum vacío y lo abre. Devuelve el error para el diálogo. */
export async function confirmarNuevoAlbum(nombre: string): Promise<string | null> {
  try {
    const album = await crearAlbum(nombre);
    estadoAlbumes.creando = false;
    await cargarAlbumes();
    abrirAlbum(album.id);
    return null;
  } catch (e) {
    return String(e);
  }
}
