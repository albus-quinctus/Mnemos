// Textos de la pantalla Álbumes (spec 014), sin estado, para probarlos solos.

const numero = (n: number) => n.toLocaleString("es");

/** «13 álbumes», «1 álbum» o «Ningún álbum». */
export function textoAlbumes(n: number): string {
  if (n === 0) return "Ningún álbum";
  return `${numero(n)} ${n === 1 ? "álbum" : "álbumes"}`;
}

/** «48 fotos», «1 foto» o «Vacío». */
export function textoFotos(n: number): string {
  if (n === 0) return "Vacío";
  return `${numero(n)} ${n === 1 ? "foto" : "fotos"}`;
}

/** Quita espacios de los extremos; `null` si no queda nada (no se puede crear). */
export function nombreValido(nombre: string): string | null {
  const limpio = nombre.trim();
  return limpio === "" ? null : limpio;
}

/** Máximo de caracteres del nombre, como `MAX_ALBUM_NAME` en el núcleo. */
export const MAX_NOMBRE = 100;

const diaMes = (d: Date) => d.toLocaleDateString("es", { day: "numeric", month: "short" });

/**
 * Rango de fechas de la cabecera de un álbum, en hora local: «7 sept 2024»,
 * «18 ago – 7 sept 2024» o «3 dic 2023 – 5 ene 2024». Vacío sin fotos.
 */
export function rangoFechas(primera: number | null, ultima: number | null): string {
  if (primera === null || ultima === null) return "";
  const a = new Date(primera * 1000);
  const b = new Date(ultima * 1000);
  if (a.toDateString() === b.toDateString()) return `${diaMes(a)} ${a.getFullYear()}`;
  if (a.getFullYear() === b.getFullYear()) return `${diaMes(a)} – ${diaMes(b)} ${b.getFullYear()}`;
  return `${diaMes(a)} ${a.getFullYear()} – ${diaMes(b)} ${b.getFullYear()}`;
}

/** «48 fotos · 18 ago – 7 sept 2024», o solo el número si no hay fechas. */
export function subtituloAlbum(fotos: number, primera: number | null, ultima: number | null) {
  const rango = rangoFechas(primera, ultima);
  return rango ? `${textoFotos(fotos)} · ${rango}` : textoFotos(fotos);
}

/** A partir de cuántos álbumes la lista de «Añadir a álbum» trae buscador. */
export const ALBUMES_SIN_BUSCADOR = 8;

const sinTildes = (t: string) =>
  t.normalize("NFD").replace(/\p{M}/gu, "").toLocaleLowerCase("es");

/** Filtra por nombre sin distinguir mayúsculas ni tildes. */
export function filtrarAlbumes<T extends { name: string }>(lista: T[], texto: string): T[] {
  const buscado = sinTildes(texto.trim());
  return buscado ? lista.filter((a) => sinTildes(a.name).includes(buscado)) : lista;
}
