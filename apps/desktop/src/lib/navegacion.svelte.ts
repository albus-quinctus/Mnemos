// Sección que se ve en el panel principal. El menú lateral la cambia.
// La spec 006 añadirá "favoritos" y "archivo".
import { limpiarBusqueda } from "$lib/busqueda/busqueda.svelte";

export type Seccion = "fotos" | "duplicados";

export const navegacion = $state({ seccion: "fotos" as Seccion });

/** Cambia de sección. Una búsqueda en marcha se cierra. */
export function irA(seccion: Seccion) {
  limpiarBusqueda();
  navegacion.seccion = seccion;
}
