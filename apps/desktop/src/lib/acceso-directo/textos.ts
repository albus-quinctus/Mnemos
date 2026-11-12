// Textos de la entrada «Añadir a este equipo» (spec 012).
import type { EstadoAccesoDirecto, SistemaAccesoDirecto } from "../biblioteca";

// Dónde aparece MNEMOS según el sistema.
function lugar(sistema: SistemaAccesoDirecto): string {
  return sistema === "windows" ? "en Inicio y en el escritorio" : "en el menú de aplicaciones";
}

/** Línea pequeña bajo la entrada. */
export function pista(sistema: SistemaAccesoDirecto, estado: EstadoAccesoDirecto): string {
  if (estado === "si") return `Ya está ${lugar(sistema)}.`;
  if (estado === "ajeno") {
    return "Ya hay un acceso directo con ese nombre que no es de MNEMOS; no se toca.";
  }
  return `MNEMOS aparecerá ${lugar(sistema)}.`;
}

export const AVISO_ANADIDO = "MNEMOS está en el menú de este equipo";
export const AVISO_QUITADO = "Se ha quitado MNEMOS de este equipo";
