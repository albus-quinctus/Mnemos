// Pilas (spec 023): textos y la pila abierta.
import type { TipoPila } from "$lib/biblioteca";

/** Cómo se dice de dónde salió una pila, en la tarjeta y en Detalles. */
export function nombreTipo(tipo: TipoPila): string {
  switch (tipo) {
    case "raw_jpeg":
      return "RAW + JPEG";
    case "editada":
      return "Editada";
    case "rafaga":
      return "Ráfaga";
    case "manual":
      return "Pila";
  }
}

/** «3 fotos», «1 foto». */
export function textoFotos(n: number): string {
  return `${n.toLocaleString("es")} ${n === 1 ? "foto" : "fotos"}`;
}
