// Sección que se ve en el panel principal. El menú lateral la cambia.
import { limpiarBusqueda } from "$lib/busqueda/busqueda.svelte";
import { limpiarSeleccion } from "$lib/seleccion/seleccion.svelte";

export type Seccion = "fotos" | "favoritos" | "archivo" | "papelera" | "duplicados";

export const navegacion = $state({ seccion: "fotos" as Seccion });

/** Cambia de sección. Una búsqueda en marcha y la selección se pierden. */
export function irA(seccion: Seccion) {
  limpiarBusqueda();
  limpiarSeleccion();
  navegacion.seccion = seccion;
}
