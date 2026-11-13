// Ventana de Ajustes (spec 013): se abre desde el menú lateral, sin cambiar de
// sección. Los cambios se aplican al momento; no hay botón «Guardar».

import { estaEnPantallaCompleta, type Tema } from "../biblioteca";
import { pantallaCompleta } from "./pantalla";
import { elegirTema, temaActual } from "./tema.svelte";

export const ajustes = $state({
  abierta: false,
  tema: "sistema" as Tema,
  pantallaCompleta: false,
});

/** Abre la ventana con el tema de ahora y el estado real de la ventana. */
export async function abrirAjustes() {
  ajustes.tema = temaActual();
  ajustes.abierta = true;
  try {
    ajustes.pantallaCompleta = await estaEnPantallaCompleta();
  } catch {
    // Fuera de Tauri: el interruptor se queda como está.
  }
}

export function cerrarAjustes() {
  ajustes.abierta = false;
}

export async function cambiarTema(tema: Tema) {
  ajustes.tema = tema;
  await elegirTema(tema);
}

/** El interruptor; F11 la alterna también desde `+layout.svelte`. */
export async function cambiarPantallaCompleta() {
  const estado = await pantallaCompleta();
  if (estado !== null) ajustes.pantallaCompleta = estado;
}
