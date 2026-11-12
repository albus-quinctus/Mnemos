// Títulos de día (spec 006), compartidos por todas las cuadrículas: el título
// es del día, no de las fotos, y se ve igual en Fotos, Favoritos, Archivo y
// la búsqueda. Se leen una vez al abrir la biblioteca.

import { SvelteMap } from "svelte/reactivity";
import { ponerTituloDia, titulosDia } from "../biblioteca";

export const MAX_TITULO = 100;

export const titulos = new SvelteMap<string, string>();

let cargados: Promise<void> | null = null;

/** Lee los títulos del núcleo, una sola vez hasta `olvidarTitulos`. */
export function cargarTitulos(): Promise<void> {
  cargados ??= titulosDia().then((lista) => {
    titulos.clear();
    for (const { day, title } of lista) titulos.set(day, title);
  });
  cargados.catch(() => (cargados = null));
  return cargados;
}

/** Al cambiar de biblioteca, los títulos se vuelven a leer. */
export function olvidarTitulos() {
  cargados = null;
  titulos.clear();
}

/**
 * Guarda el título de un día. El núcleo lo limpia (saltos de línea, 100
 * caracteres) y devuelve cómo queda; vacío lo borra. Un error sube tal cual
 * y el título anterior se conserva.
 */
export async function guardarTitulo(dia: string, texto: string): Promise<void> {
  const guardado = await ponerTituloDia(dia, texto);
  if (guardado) titulos.set(dia, guardado);
  else titulos.delete(dia);
}
