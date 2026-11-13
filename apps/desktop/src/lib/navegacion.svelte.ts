// Sección que se ve en el panel principal. El menú lateral la cambia.
import { limpiarBusqueda } from "$lib/busqueda/busqueda.svelte";
import { limpiarSeleccion } from "$lib/seleccion/seleccion.svelte";

export type Seccion =
  | "fotos"
  | "albumes"
  | "etiquetas"
  | "favoritos"
  | "archivo"
  | "papelera"
  | "duplicados";

// En la sección Álbumes, `album` es el álbum abierto, o `null` para ver la
// lista (spec 014). Lo mismo con `etiqueta` en la sección Etiquetas (spec 016).
export const navegacion = $state({
  seccion: "fotos" as Seccion,
  album: null as number | null,
  etiqueta: null as number | null,
});

/** Cambia de sección. Una búsqueda en marcha y la selección se pierden. */
export function irA(seccion: Seccion) {
  limpiarBusqueda();
  limpiarSeleccion();
  navegacion.seccion = seccion;
  navegacion.album = null;
  navegacion.etiqueta = null;
}

/** Abre un álbum dentro de la sección Álbumes. */
export function abrirAlbum(id: number) {
  limpiarBusqueda();
  limpiarSeleccion();
  navegacion.seccion = "albumes";
  navegacion.album = id;
  navegacion.etiqueta = null;
}

/** Abre las fotos de una etiqueta dentro de la sección Etiquetas. */
export function abrirEtiqueta(id: number) {
  limpiarBusqueda();
  limpiarSeleccion();
  navegacion.seccion = "etiquetas";
  navegacion.album = null;
  navegacion.etiqueta = id;
}
