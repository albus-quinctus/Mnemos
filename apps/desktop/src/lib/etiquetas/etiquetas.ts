// Textos y reglas de las etiquetas (spec 016), sin estado, para probarlos solos.

const numero = (n: number) => n.toLocaleString("es");

/** Máximo de caracteres del nombre, como `MAX_TAG_NAME` en el núcleo. */
export const MAX_NOMBRE = 50;

/** «11 etiquetas», «1 etiqueta» o «Ninguna etiqueta». */
export function textoEtiquetas(n: number): string {
  if (n === 0) return "Ninguna etiqueta";
  return `${numero(n)} ${n === 1 ? "etiqueta" : "etiquetas"}`;
}

/** «318 fotos», «1 foto» o «0 fotos». */
export function textoFotos(n: number): string {
  return `${numero(n)} ${n === 1 ? "foto" : "fotos"}`;
}

/**
 * Nombre como lo guarda el núcleo: sin espacios en los extremos y con los de
 * en medio reducidos a uno. `null` si queda vacío o pasa de 50 caracteres.
 */
export function nombreValido(nombre: string): string | null {
  // eslint-disable-next-line no-control-regex
  const limpio = nombre.replace(/[\s\u0000-\u001f\u007f]+/gu, " ").trim();
  if (limpio === "" || [...limpio].length > MAX_NOMBRE) return null;
  return limpio;
}

const orden = new Intl.Collator("es", { sensitivity: "base", numeric: true });

/** Por nombre, con las reglas del español: «Ávila» junto a «Avila». */
export function ordenarEtiquetas<T extends { name: string }>(lista: T[]): T[] {
  return [...lista].sort((a, b) => orden.compare(a.name, b.name));
}

const sinTildes = (t: string) =>
  t.normalize("NFD").replace(/\p{M}/gu, "").toLocaleLowerCase("es");

/**
 * Las etiquetas cuyo nombre contiene `texto` (sin mayúsculas ni tildes), con
 * las que empiezan por él delante.
 */
export function filtrarEtiquetas<T extends { name: string }>(lista: T[], texto: string): T[] {
  const buscado = sinTildes(texto.trim());
  if (!buscado) return lista;
  const empiezan: T[] = [];
  const contienen: T[] = [];
  for (const e of lista) {
    const nombre = sinTildes(e.name);
    if (nombre.startsWith(buscado)) empiezan.push(e);
    else if (nombre.includes(buscado)) contienen.push(e);
  }
  return [...empiezan, ...contienen];
}

/** Si ya hay una etiqueta que se llama exactamente así, sin mirar mayúsculas. */
export function existeNombre<T extends { name: string }>(lista: T[], nombre: string): boolean {
  const buscado = nombre.trim().toLocaleLowerCase("es");
  return lista.some((e) => e.name.toLocaleLowerCase("es") === buscado);
}

export type Casilla = "si" | "algo" | "no";

/** Marcada si todas las seleccionadas la tienen, guion si algunas. */
export function casilla(conEtiqueta: number, seleccionadas: number): Casilla {
  if (conEtiqueta <= 0) return "no";
  return conEtiqueta >= seleccionadas ? "si" : "algo";
}

/** Un cambio hecho desde el desplegable «Etiquetar», para el aviso y «Deshacer». */
export interface CambioEtiqueta {
  nombre: string;
  puesta: boolean;
  /** Las fotos que cambiaron de verdad. */
  fotos: number[];
}

const fotos = (n: number) => (n === 1 ? "1 foto" : `${numero(n)} fotos`);

/** Texto del aviso al cerrar el desplegable, o `null` si nada cambió. */
export function resumenCambios(cambios: CambioEtiqueta[]): string | null {
  const reales = cambios.filter((c) => c.fotos.length > 0);
  if (reales.length === 0) return null;
  if (reales.length === 1) {
    const c = reales[0];
    return c.puesta
      ? `Etiqueta «${c.nombre}» añadida a ${fotos(c.fotos.length)}`
      : `Etiqueta «${c.nombre}» quitada de ${fotos(c.fotos.length)}`;
  }
  const todas = new Set(reales.flatMap((c) => c.fotos));
  return `Etiquetas cambiadas en ${fotos(todas.size)}`;
}
