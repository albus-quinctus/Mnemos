// Disposición de la cuadrícula justificada (spec 002): filas de altura
// parecida, cada foto con su proporción y sin recortes, agrupadas por mes.
// Es una función pura para poder probarla sin navegador.

import type { FotoCuadricula, Mes } from "../biblioteca";

export interface Opciones {
  ancho: number; // ancho disponible en píxeles
  altoFila: number; // alto objetivo de cada fila
  separacion: number; // hueco entre fotos y entre filas
  altoMes: number; // alto del encabezado de cada mes
}

export interface Celda {
  indice: number; // posición en la lista de fotos cargadas
  x: number;
  ancho: number;
}

export type Bloque =
  | { tipo: "mes"; mes: string; y: number; alto: number }
  | { tipo: "fila"; y: number; alto: number; celdas: Celda[] }
  // Fotos del mes que aún no se han pedido al núcleo: se reserva su alto
  // estimado para que la barra de desplazamiento sea fiel desde el principio.
  | { tipo: "pendiente"; mes: string; y: number; alto: number; fotos: number };

export interface Disposicion {
  bloques: Bloque[];
  alto: number;
}

// Proporción supuesta cuando falta y para estimar lo que no se ha cargado.
const PROPORCION_DESCONOCIDA = 1;
const PROPORCION_MEDIA = 1.4;
// Topes para que una panorámica o una foto muy estrecha no rompan la fila. El
// máximo coincide con el tope de las miniaturas (1080 × 360).
const PROPORCION_MIN = 0.5;
const PROPORCION_MAX = 3;

export function proporcion(foto: Pick<FotoCuadricula, "width" | "height">): number {
  const { width, height } = foto;
  if (!width || !height) return PROPORCION_DESCONOCIDA;
  return Math.min(PROPORCION_MAX, Math.max(PROPORCION_MIN, width / height));
}

/** Las fotos llegan en el orden de los meses, de lo más reciente a lo más antiguo. */
export function disponer(meses: Mes[], fotos: FotoCuadricula[], op: Opciones): Disposicion {
  const bloques: Bloque[] = [];
  const ancho = Math.max(op.ancho, op.altoFila * PROPORCION_MIN);
  let y = 0;
  let i = 0;

  for (const mes of meses) {
    bloques.push({ tipo: "mes", mes: mes.month, y, alto: op.altoMes });
    y += op.altoMes;

    const inicio = i;
    while (i < fotos.length && fotos[i].month === mes.month) i++;
    for (const fila of justificar(fotos, inicio, i, ancho, op)) {
      bloques.push({ ...fila, y });
      y += fila.alto + op.separacion;
    }

    const faltan = mes.photos - (i - inicio);
    if (faltan > 0) {
      const porFila = Math.max(1, Math.floor(ancho / (op.altoFila * PROPORCION_MEDIA)));
      const alto = Math.ceil(faltan / porFila) * (op.altoFila + op.separacion);
      bloques.push({ tipo: "pendiente", mes: mes.month, y, alto, fotos: faltan });
      y += alto;
    }
  }
  return { bloques, alto: y };
}

function justificar(
  fotos: FotoCuadricula[],
  desde: number,
  hasta: number,
  ancho: number,
  op: Opciones,
): { tipo: "fila"; alto: number; celdas: Celda[] }[] {
  const filas: { tipo: "fila"; alto: number; celdas: Celda[] }[] = [];
  let fila: number[] = [];
  let suma = 0;

  const cerrar = (alto: number, estirar: boolean) => {
    const celdas: Celda[] = [];
    let x = 0;
    fila.forEach((indice, n) => {
      const ultima = n === fila.length - 1;
      // La última celda de una fila completa llega justo al borde, sin
      // dejar medio píxel suelto por redondeos.
      const w = ultima && estirar ? ancho - x : proporcion(fotos[indice]) * alto;
      celdas.push({ indice, x, ancho: w });
      x += w + op.separacion;
    });
    filas.push({ tipo: "fila", alto, celdas });
    fila = [];
    suma = 0;
  };

  for (let i = desde; i < hasta; i++) {
    fila.push(i);
    suma += proporcion(fotos[i]);
    const huecos = (fila.length - 1) * op.separacion;
    if (suma * op.altoFila + huecos >= ancho) {
      cerrar((ancho - huecos) / suma, true);
    }
  }
  // La última fila del mes no se estira: conserva el alto objetivo.
  if (fila.length > 0) cerrar(op.altoFila, false);
  return filas;
}

/** Bloques que se ven entre `desde` y `hasta` (en píxeles desde arriba). */
export function visibles(bloques: Bloque[], desde: number, hasta: number): Bloque[] {
  // Búsqueda binaria del primer bloque que acaba después de `desde`.
  let lo = 0;
  let hi = bloques.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (bloques[mid].y + bloques[mid].alto < desde) lo = mid + 1;
    else hi = mid;
  }
  const salida: Bloque[] = [];
  for (let i = lo; i < bloques.length && bloques[i].y <= hasta; i++) salida.push(bloques[i]);
  return salida;
}

/** "2026-10" → "Octubre de 2026". */
export function nombreMes(mes: string): string {
  const [anio, numero] = mes.split("-").map(Number);
  const texto = new Date(anio, numero - 1, 1).toLocaleDateString("es", {
    month: "long",
    year: "numeric",
  });
  return texto.charAt(0).toUpperCase() + texto.slice(1);
}
