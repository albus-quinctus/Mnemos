// Disposición de la cuadrícula justificada (spec 002), como en Google Fotos:
// las fotos se agrupan por día con un título pequeño encima, los días con
// pocas fotos comparten fila y el mes solo se ve en la barra de la derecha.
// Es una función pura para poder probarla sin navegador.

import type { FotoCuadricula, Mes } from "../biblioteca";

export interface Opciones {
  ancho: number; // ancho disponible en píxeles
  altoFila: number; // alto objetivo de las fotos de cada fila
  separacion: number; // hueco entre fotos y entre filas
  separacionDias: number; // hueco entre dos días que comparten fila
  altoTitulo: number; // alto del título del día, encima de sus fotos
}

export interface Celda {
  indice: number; // posición en la lista de fotos cargadas
  x: number;
  ancho: number;
}

export interface Titulo {
  dia: string; // "2026-10-20", en hora local
  x: number;
}

export type Bloque =
  // Una fila de fotos. Si en ella empieza algún día, lleva encima su título y
  // las fotos bajan `altoTitulo`.
  | { tipo: "fila"; y: number; alto: number; altoTitulo: number; titulos: Titulo[]; celdas: Celda[] }
  // Fotos del mes que aún no se han pedido al núcleo: se reserva su alto
  // estimado para que la barra de desplazamiento sea fiel desde el principio.
  | { tipo: "pendiente"; mes: string; y: number; alto: number; fotos: number };

type BloqueFila = Extract<Bloque, { tipo: "fila" }>;

/** Dónde empieza cada mes, para la barra de meses y años. */
export interface Marca {
  mes: string;
  y: number;
}

export interface Disposicion {
  bloques: Bloque[];
  marcas: Marca[];
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

/** Día local de una foto: "2026-10-20". */
export function diaDe(foto: Pick<FotoCuadricula, "date">): string {
  const d = new Date(foto.date * 1000);
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mm}-${dd}`;
}

interface FilaDia {
  celdas: Celda[];
  alto: number;
  anchoUsado: number;
  completa: boolean; // llega al borde derecho
}

/** Las fotos llegan en el orden de los meses, de lo más reciente a lo más antiguo. */
export function disponer(meses: Mes[], fotos: FotoCuadricula[], op: Opciones): Disposicion {
  const bloques: Bloque[] = [];
  const marcas: Marca[] = [];
  const ancho = Math.max(op.ancho, op.altoFila * PROPORCION_MIN);
  let y = 0;
  let i = 0;

  // Fila abierta en la que aún caben días cortos.
  // (Con `as` para que TypeScript no la dé por siempre nula: la cambia `cerrarAbierta`.)
  let abierta = null as BloqueFila | null;
  let xLibre = 0;
  const cerrarAbierta = () => {
    if (abierta) {
      bloques.push(abierta);
      y += abierta.alto + op.separacion;
      abierta = null;
    }
  };

  for (const mes of meses) {
    const inicio = i;
    while (i < fotos.length && fotos[i].month === mes.month) i++;
    if (i > inicio || mes.photos > 0) {
      // El mes empieza donde empiece su primer día.
      marcas.push({ mes: mes.month, y: abierta ? abierta.y : y });
    }

    let d = inicio;
    while (d < i) {
      const dia = diaDe(fotos[d]);
      const desde = d;
      while (d < i && diaDe(fotos[d]) === dia) d++;
      const filas = justificar(fotos, desde, d, ancho, op);

      // Un día de una sola fila corta cabe junto al anterior si hay sitio.
      const corto = filas.length === 1 && !filas[0].completa;
      if (abierta && corto && xLibre + op.separacionDias + filas[0].anchoUsado <= ancho) {
        const x0 = xLibre + op.separacionDias;
        abierta.titulos.push({ dia, x: x0 });
        abierta.celdas.push(...filas[0].celdas.map((c) => ({ ...c, x: c.x + x0 })));
        xLibre = x0 + filas[0].anchoUsado;
        continue;
      }

      cerrarAbierta();
      filas.forEach((fila, n) => {
        const bloque: BloqueFila = {
          tipo: "fila",
          y,
          alto: (n === 0 ? op.altoTitulo : 0) + fila.alto,
          altoTitulo: n === 0 ? op.altoTitulo : 0,
          titulos: n === 0 ? [{ dia, x: 0 }] : [],
          celdas: fila.celdas,
        };
        if (n === filas.length - 1 && !fila.completa && n === 0) {
          // Fila corta de un día: queda abierta para los días siguientes.
          abierta = bloque;
          xLibre = fila.anchoUsado;
        } else {
          bloques.push(bloque);
          y += bloque.alto + op.separacion;
        }
      });
    }

    const faltan = mes.photos - (i - inicio);
    if (faltan > 0) {
      cerrarAbierta();
      const porFila = Math.max(1, Math.floor(ancho / (op.altoFila * PROPORCION_MEDIA)));
      const alto = Math.ceil(faltan / porFila) * (op.altoFila + op.separacion) + op.altoTitulo;
      bloques.push({ tipo: "pendiente", mes: mes.month, y, alto, fotos: faltan });
      y += alto;
    }
  }
  cerrarAbierta();
  return { bloques, marcas, alto: y };
}

function justificar(
  fotos: FotoCuadricula[],
  desde: number,
  hasta: number,
  ancho: number,
  op: Opciones,
): FilaDia[] {
  const filas: FilaDia[] = [];
  let fila: number[] = [];
  let suma = 0;

  const cerrar = (alto: number, completa: boolean) => {
    const celdas: Celda[] = [];
    let x = 0;
    fila.forEach((indice, n) => {
      const ultima = n === fila.length - 1;
      // La última celda de una fila completa llega justo al borde, sin
      // dejar medio píxel suelto por redondeos.
      const w = ultima && completa ? ancho - x : proporcion(fotos[indice]) * alto;
      celdas.push({ indice, x, ancho: w });
      x += w + op.separacion;
    });
    filas.push({ celdas, alto, anchoUsado: x - op.separacion, completa });
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
  // La última fila del día no se estira: conserva el alto objetivo.
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

/** Mes de la marca que hay en `y`: el último que empieza antes. */
export function mesEn(marcas: Marca[], y: number): string | undefined {
  let mes = marcas[0]?.mes;
  for (const marca of marcas) {
    if (marca.y > y) break;
    mes = marca.mes;
  }
  return mes;
}

/** "2026-10" → "Octubre de 2026". */
export function nombreMes(mes: string): string {
  const [anio, numero] = mes.split("-").map(Number);
  const texto = new Date(anio, numero - 1, 1).toLocaleDateString("es", {
    month: "long",
    year: "numeric",
  });
  return mayuscula(texto);
}

/** "2026-10-20" → "Lun, 20 oct"; de otros años, "Lun, 20 oct 2025". */
export function nombreDia(dia: string, hoy = new Date()): string {
  const [anio, mes, numero] = dia.split("-").map(Number);
  const fecha = new Date(anio, mes - 1, numero);
  const texto = fecha.toLocaleDateString("es", {
    weekday: "short",
    day: "numeric",
    month: "short",
    year: anio === hoy.getFullYear() ? undefined : "numeric",
  });
  return mayuscula(texto.replace(/\./g, ""));
}

function mayuscula(texto: string): string {
  return texto.charAt(0).toUpperCase() + texto.slice(1);
}
