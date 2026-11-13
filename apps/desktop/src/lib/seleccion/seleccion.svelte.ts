// Estado de la selección múltiple (spec 006). Vive solo en la interfaz: una
// lista de ids que nunca se guarda en la base de datos. Los ids de un día, de
// toda la vista o de un rango salen del núcleo, no de las páginas cargadas.

import { SvelteSet } from "svelte/reactivity";
import {
  seleccionarDia,
  seleccionarRango,
  seleccionarTodo,
  type Ambito,
  type Cursor,
} from "../biblioteca";

/** Lo que la cuadrícula sabe de una foto para seleccionarla. */
export interface FotoSeleccionable {
  id: number;
  date: number;
}

/** Teclas modificadoras de un clic o de una pulsación. */
export interface Modificadores {
  ctrl?: boolean; // Ctrl, o Cmd en macOS
  mayus?: boolean;
}

export const seleccion = $state({
  ids: new SvelteSet<number>(),
  // Última foto marcada o desmarcada: un extremo del rango de Mayús + clic.
  ancla: null as Cursor | null,
  ambito: { vista: "fotos" } as Ambito,
});

// Sube con cada cambio de ámbito o limpieza: una respuesta del núcleo que
// llega después pertenece a una selección que ya no existe y se descarta.
let generacion = 0;

export function haySeleccion(): boolean {
  return seleccion.ids.size > 0;
}

export function estaSeleccionada(id: number): boolean {
  return seleccion.ids.has(id);
}

/** Si todas las fotos dadas están seleccionadas (para la casilla de un día). */
export function todasSeleccionadas(ids: readonly number[]): boolean {
  return ids.length > 0 && ids.every((id) => seleccion.ids.has(id));
}

/** Quita la selección. */
export function limpiarSeleccion() {
  generacion++;
  seleccion.ids.clear();
  seleccion.ancla = null;
}

/** Cambiar de vista, buscar o salir de la búsqueda pierde la selección. */
export function cambiarAmbito(ambito: Ambito) {
  if (mismoAmbito(ambito, seleccion.ambito)) return;
  limpiarSeleccion();
  seleccion.ambito = ambito;
}

function mismoAmbito(a: Ambito, b: Ambito): boolean {
  // Un álbum o una etiqueta es un objeto: se comparan por su contenido.
  if ("vista" in a) return "vista" in b && JSON.stringify(a.vista) === JSON.stringify(b.vista);
  return "busqueda" in b && a.busqueda === b.busqueda;
}

/** Marca o desmarca una foto y la deja como ancla del rango. */
export function alternar(foto: FotoSeleccionable) {
  if (!seleccion.ids.delete(foto.id)) seleccion.ids.add(foto.id);
  seleccion.ancla = { date: foto.date, id: foto.id };
}

/**
 * Selecciona todas las fotos entre el ancla y esta, en el orden de la
 * cuadrícula y aunque haya páginas sin cargar en medio. Sin ancla, se
 * comporta como Ctrl + clic.
 */
export async function seleccionarHasta(foto: FotoSeleccionable) {
  const desde = seleccion.ancla;
  if (!desde) return alternar(foto);
  const hasta = { date: foto.date, id: foto.id };
  const ids = await conGeneracion(() =>
    seleccionarRango(seleccion.ambito, desde, hasta),
  );
  if (!ids) return;
  for (const id of ids) seleccion.ids.add(id);
  seleccion.ancla = hasta;
}

/** Ctrl + A: todas las fotos de la vista o de la búsqueda actual. */
export async function seleccionarVista() {
  const ids = await conGeneracion(() => seleccionarTodo(seleccion.ambito));
  if (!ids) return;
  for (const id of ids) seleccion.ids.add(id);
}

/**
 * Casilla de la cabecera de un día: si ya estaban todas sus fotos, las quita;
 * si no, las añade todas.
 */
export async function alternarDia(dia: string) {
  const ids = await conGeneracion(() => seleccionarDia(seleccion.ambito, dia));
  if (!ids) return;
  if (todasSeleccionadas(ids)) {
    for (const id of ids) seleccion.ids.delete(id);
  } else {
    for (const id of ids) seleccion.ids.add(id);
  }
}

// Pide ids al núcleo y devuelve null si entretanto cambió la selección. Un
// error (disco desconectado) sube tal cual y la selección no se toca.
async function conGeneracion(
  pedir: () => Promise<number[]>,
): Promise<number[] | null> {
  const mia = generacion;
  const ids = await pedir();
  return mia === generacion ? ids : null;
}

/**
 * Clic en una miniatura. Devuelve true si el clic era de selección y false si
 * debe abrir el visor como siempre.
 */
export function clicEnFoto(
  foto: FotoSeleccionable,
  mods: Modificadores = {},
): boolean {
  if (mods.mayus) {
    void seleccionarHasta(foto).catch(avisarError);
    return true;
  }
  if (mods.ctrl || haySeleccion()) {
    alternar(foto);
    return true;
  }
  return false;
}

/** Pulsación de tecla de la cuadrícula. Devuelve true si la ha usado. */
export function teclaSeleccion(
  tecla: string,
  mods: Modificadores = {},
): boolean {
  if (tecla === "Escape") {
    if (!haySeleccion()) return false; // Esc hace lo de siempre
    limpiarSeleccion();
    return true;
  }
  if (mods.ctrl && (tecla === "a" || tecla === "A") && !mods.mayus) {
    void seleccionarVista().catch(avisarError);
    return true;
  }
  return false;
}

// Los errores de las acciones lanzadas desde un clic o una tecla se avisan
// aquí; la barra de selección (tarea 10) los mostrará.
let avisarError: (error: unknown) => void = (error) => console.error(error);

export function alFallar(avisar: (error: unknown) => void) {
  avisarError = avisar;
}

/** Para las acciones que la cuadrícula lanza sin esperar su resultado. */
export function avisarFallo(error: unknown) {
  avisarError(error);
}
