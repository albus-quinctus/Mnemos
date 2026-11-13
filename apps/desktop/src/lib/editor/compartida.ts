// Textos del aviso de ediciones compartidas entre copias idénticas
// (spec 018). Las fotos con la misma huella comparten la receta: lo que se
// guarda como original se ve en todas.
import type { CopiaDuplicada } from "$lib/biblioteca";

/** Cuántas rutas enseña el diálogo antes de «y N más». */
export const MAX_RUTAS = 3;

export function tituloAviso(copias: number): string {
  return copias === 1
    ? "Esta foto tiene otra copia idéntica"
    : `Esta foto tiene otras ${copias} copias idénticas`;
}

export function botonAplicar(copias: number): string {
  return copias === 1 ? "Aplicar a las dos" : "Aplicar a todas";
}

export interface RutaAviso {
  texto: string;
  desaparecida: boolean;
}

/** Rutas que se enseñan, sin el prefijo `Originales/`, y cuántas quedan. */
export function rutasAviso(
  copias: CopiaDuplicada[],
  max = MAX_RUTAS,
): { rutas: RutaAviso[]; resto: number } {
  // Con una más de la cuenta, se enseña entera en vez de «y 1 más».
  const n = copias.length <= max + 1 ? copias.length : max;
  return {
    rutas: copias.slice(0, n).map((c) => ({
      texto: c.rel_path.replace(/^Originales\//, ""),
      desaparecida: !c.exists,
    })),
    resto: copias.length - n,
  };
}

/** Línea fija de la sección Ediciones en Detalles, o null sin copias. */
export function lineaCompartida(copias: number): string | null {
  if (copias <= 0) return null;
  return copias === 1
    ? "Compartidas con 1 copia idéntica"
    : `Compartidas con ${copias} copias idénticas`;
}
