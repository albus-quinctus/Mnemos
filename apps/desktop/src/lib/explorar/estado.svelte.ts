// Estado de la pantalla Explorar (spec 021): las tres filas que calcula el
// núcleo. Solo se leen datos; nada cambia en la biblioteca.
import { explorar as pedirExplorar, type Explorar } from "$lib/biblioteca";

export const estadoExplorar = $state({
  datos: null as Explorar | null,
  cargado: false,
  error: "",
});

/** Vuelve a pedir las filas (al entrar y después de cada cambio). */
export async function cargarExplorar() {
  try {
    estadoExplorar.datos = await pedirExplorar();
    estadoExplorar.error = "";
  } catch (e) {
    estadoExplorar.error = String(e);
  } finally {
    estadoExplorar.cargado = true;
  }
}

/** Ninguna fila tiene tarjetas. */
export function sinFilas(datos: Explorar): boolean {
  return datos.memories.length === 0 && datos.collections.length === 0 && datos.tags.length === 0;
}
