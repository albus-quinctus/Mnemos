// Tema de la interfaz (spec 013). Rust lo aplica al crear la ventana
// (`data-tema` en `<html>`, antes de cargar la página); aquí se lee y se
// cambia durante la sesión. Con «sistema» no hay atributo y manda el CSS.

import { ponerTema, type Tema } from "../biblioteca";
import { mostrarAviso } from "../seleccion/acciones.svelte";

export const AVISO_NO_GUARDADO = "No se pudo guardar el ajuste";

/** Pone `data-tema` en `<html>`; con «sistema» lo quita. */
export function aplicarTema(
  tema: Tema,
  raiz: HTMLElement = document.documentElement,
) {
  if (tema === "sistema") delete raiz.dataset.tema;
  else raiz.dataset.tema = tema;
}

/** El tema que hay ahora en `<html>`. */
export function temaActual(raiz: HTMLElement = document.documentElement): Tema {
  const tema = raiz.dataset.tema;
  return tema === "claro" || tema === "oscuro" ? tema : "sistema";
}

/** Cambia el tema al momento y lo guarda; si no se puede guardar, sigue
 *  aplicado en esta sesión y se avisa. */
export async function elegirTema(
  tema: Tema,
  raiz: HTMLElement = document.documentElement,
) {
  aplicarTema(tema, raiz);
  try {
    await ponerTema(tema);
  } catch {
    mostrarAviso({ texto: AVISO_NO_GUARDADO, error: true });
  }
}
