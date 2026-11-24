// Sección que se ve en el panel principal. El menú lateral la cambia.
import type { Vista } from "$lib/biblioteca";
import { limpiarBusqueda } from "$lib/busqueda/busqueda.svelte";
import { limpiarSeleccion } from "$lib/seleccion/seleccion.svelte";

export type Seccion =
  | "fotos"
  | "explorar"
  | "albumes"
  | "etiquetas"
  | "favoritos"
  | "archivo"
  | "papelera"
  | "duplicados";

// En la sección Álbumes, `album` es el álbum abierto, o `null` para ver la
// lista (spec 014). Lo mismo con `etiqueta` en la sección Etiquetas (spec 016)
// y con `explorar` en Explorar (spec 021): la colección o el recuerdo abierto.
export const navegacion = $state({
  seccion: "fotos" as Seccion,
  album: null as number | null,
  etiqueta: null as number | null,
  explorar: null as AbiertoEnExplorar | null,
  // La pila abierta (spec 023), encima de cualquier sección; `null` si no hay.
  pila: null as number | null,
});

/** Lo abierto desde Explorar: su vista y el título de su cabecera. */
export interface AbiertoEnExplorar {
  vista: Vista;
  titulo: string;
  /** Texto tras el número de elementos, como el rango de un recuerdo. */
  detalle?: string;
}

/** Cambia de sección. Una búsqueda en marcha y la selección se pierden. */
export function irA(seccion: Seccion) {
  limpiarBusqueda();
  limpiarSeleccion();
  navegacion.seccion = seccion;
  navegacion.album = null;
  navegacion.etiqueta = null;
  navegacion.explorar = null;
  navegacion.pila = null;
}

/** Abre una pila encima de lo que se ve (spec 023). La selección se pierde. */
export function abrirPila(id: number) {
  limpiarSeleccion();
  navegacion.pila = id;
}

/** Vuelve de la pila a donde se estaba. */
export function cerrarPila() {
  limpiarSeleccion();
  navegacion.pila = null;
}

/** Abre una colección o un recuerdo dentro de la sección Explorar. */
export function abrirEnExplorar(abierto: AbiertoEnExplorar) {
  limpiarBusqueda();
  limpiarSeleccion();
  navegacion.seccion = "explorar";
  navegacion.album = null;
  navegacion.etiqueta = null;
  navegacion.explorar = abierto;
}

/** Abre un álbum dentro de la sección Álbumes. */
export function abrirAlbum(id: number) {
  limpiarBusqueda();
  limpiarSeleccion();
  navegacion.seccion = "albumes";
  navegacion.album = id;
  navegacion.etiqueta = null;
  navegacion.explorar = null;
}

/** Abre las fotos de una etiqueta dentro de la sección Etiquetas. */
export function abrirEtiqueta(id: number) {
  limpiarBusqueda();
  limpiarSeleccion();
  navegacion.seccion = "etiquetas";
  navegacion.album = null;
  navegacion.etiqueta = id;
  navegacion.explorar = null;
}
