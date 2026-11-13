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
