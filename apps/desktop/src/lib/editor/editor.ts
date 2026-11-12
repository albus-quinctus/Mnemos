// Estado del editor no destructivo (spec 009): recorte, giro, volteo y
// proporción, sin tocar la interfaz. Cada operación devuelve un estado nuevo.
//
// El recorte está en fracciones (0 a 1) de la foto tal como se ve en el
// editor: ya girada según el EXIF, con el giro y el volteo de la receta. Es
// el mismo marco que `geometry.crop` en la receta.

import type { Geometria, Recorte } from "../biblioteca";

export type Proporcion =
  | "libre"
  | "original"
  | "cuadrado"
  | "4:3"
  | "3:2"
  | "16:9";

export const PROPORCIONES: readonly Proporcion[] = [
  "libre",
  "original",
  "cuadrado",
  "4:3",
  "3:2",
  "16:9",
];

export type Esquina = "no" | "ne" | "so" | "se";
export type Lado = "n" | "s" | "e" | "o";

export interface EstadoEditor {
  /** Tamaño en píxeles de la foto ya girada según el EXIF, sin la receta. */
  ancho: number;
  alto: number;
  rotate: 0 | 90 | 180 | 270;
  flip_horizontal: boolean;
  /** Siempre presente: la foto entera es `{0, 0, 1, 1}`. */
  recorte: Recorte;
  proporcion: Proporcion;
  /** Proporción en vertical (3:4 en vez de 4:3). No afecta a «Cuadrado». */
  vertical: boolean;
  /** Lo que había al entrar, para saber si hay cambios sin guardar. */
  base: Geometria;
}

/** Lado mínimo del recorte, en píxeles del original. */
export const MINIMO_PX = 32;
/** Paso de las flechas, y con Mayús. */
export const PASO = 0.01;
export const PASO_LARGO = 0.1;

const ENTERA: Recorte = { x: 0, y: 0, width: 1, height: 1 };
const EPSILON = 1e-6;

/** Estado al entrar al editor sobre una foto con la geometría `base`. */
export function crearEstado(
  ancho: number,
  alto: number,
  base: Geometria = { rotate: 0, flip_horizontal: false, crop: null },
): EstadoEditor {
  return {
    ancho: Math.max(1, ancho),
    alto: Math.max(1, alto),
    rotate: base.rotate,
    flip_horizontal: base.flip_horizontal,
    recorte: base.crop ? { ...base.crop } : { ...ENTERA },
    proporcion: "libre",
    vertical: false,
    base,
  };
}

/** Tamaño en píxeles de la foto con el giro aplicado (lo que se ve). */
export function tamanoVisible(e: EstadoEditor): {
  ancho: number;
  alto: number;
} {
  return e.rotate % 180 === 0
    ? { ancho: e.ancho, alto: e.alto }
    : { ancho: e.alto, alto: e.ancho };
}

/** Tamaño en píxeles del recorte. */
export function tamanoRecorte(e: EstadoEditor): {
  ancho: number;
  alto: number;
} {
  const v = tamanoVisible(e);
  return {
    ancho: Math.max(1, Math.round(e.recorte.width * v.ancho)),
    alto: Math.max(1, Math.round(e.recorte.height * v.alto)),
  };
}

/** Ancho entre alto, en píxeles, que impone la proporción, o null si es libre. */
export function razon(e: EstadoEditor): number | null {
  const v = tamanoVisible(e);
  let r: number;
  switch (e.proporcion) {
    case "libre":
      return null;
    case "cuadrado":
      return 1;
    case "original":
      r = v.ancho / v.alto;
      break;
    case "4:3":
      r = 4 / 3;
      break;
    case "3:2":
      r = 3 / 2;
      break;
    case "16:9":
      r = 16 / 9;
      break;
  }
  return e.vertical ? 1 / r : r;
}

/** Lado mínimo del recorte en fracciones de la foto visible. */
function minimos(e: EstadoEditor): { w: number; h: number } {
  const v = tamanoVisible(e);
  return {
    w: Math.min(1, MINIMO_PX / v.ancho),
    h: Math.min(1, MINIMO_PX / v.alto),
  };
}

const limitar = (v: number, min: number, max: number) =>
  Math.min(max, Math.max(min, v));

/** Mueve el recorte sin cambiar su tamaño; nunca sale de la foto. */
export function mover(e: EstadoEditor, dx: number, dy: number): EstadoEditor {
  const r = e.recorte;
  return {
    ...e,
    recorte: {
      ...r,
      x: limitar(r.x + dx, 0, 1 - r.width),
      y: limitar(r.y + dy, 0, 1 - r.height),
    },
  };
}

/**
 * Arrastra una esquina `dx`, `dy` (en fracciones). La esquina opuesta queda
 * fija. Con una proporción elegida, el recorte la mantiene.
 */
export function arrastrarEsquina(
  e: EstadoEditor,
  esquina: Esquina,
  dx: number,
  dy: number,
): EstadoEditor {
  const r = e.recorte;
  const oeste = esquina === "no" || esquina === "so";
  const norte = esquina === "no" || esquina === "ne";
  // Esquina fija y espacio disponible desde ella hacia la que se arrastra.
  const fijoX = oeste ? r.x + r.width : r.x;
  const fijoY = norte ? r.y + r.height : r.y;
  const libreX = oeste ? fijoX : 1 - fijoX;
  const libreY = norte ? fijoY : 1 - fijoY;
  const min = minimos(e);

  let w = (oeste ? -dx : dx) + r.width;
  let h = (norte ? -dy : dy) + r.height;
  const k = razon(e);
  if (k === null) {
    w = limitar(w, min.w, libreX);
    h = limitar(h, min.h, libreY);
  } else {
    // En fracciones, la proporción en píxeles k pasa a ser h = w · f.
    const v = tamanoVisible(e);
    const f = v.ancho / (v.alto * k);
    // Manda el lado que más ha crecido, y luego cabe en el espacio libre.
    w = Math.max(w, h / f);
    w = limitar(w, Math.max(min.w, min.h / f), Math.min(libreX, libreY / f));
    h = w * f;
  }
  return {
    ...e,
    recorte: {
      x: oeste ? fijoX - w : fijoX,
      y: norte ? fijoY - h : fijoY,
      width: w,
      height: h,
    },
  };
}

/**
 * Arrastra un lado `delta` (en fracciones; positivo hacia la derecha o hacia
 * abajo, como el ratón). El lado opuesto queda fijo. Con una proporción, el
 * otro lado crece o mengua centrado.
 */
export function arrastrarLado(
  e: EstadoEditor,
  lado: Lado,
  delta: number,
): EstadoEditor {
  const r = e.recorte;
  const min = minimos(e);
  const horizontal = lado === "e" || lado === "o";
  const k = razon(e);
  let { x, y, width: w, height: h } = r;

  if (horizontal) {
    const fijo = lado === "o" ? x + w : x;
    const libre = lado === "o" ? fijo : 1 - fijo;
    w = limitar(lado === "o" ? w - delta : w + delta, min.w, libre);
    if (k !== null) {
      const v = tamanoVisible(e);
      const f = v.ancho / (v.alto * k);
      const centro = y + h / 2;
      // El alto cabe centrado si puede; si no, limita también el ancho.
      const maxH = 2 * Math.min(centro, 1 - centro);
      w = limitar(
        w,
        Math.max(min.w, min.h / f),
        Math.min(libre, Math.max(maxH, h) / f),
      );
      h = w * f;
      y = limitar(centro - h / 2, 0, 1 - h);
    }
    x = lado === "o" ? fijo - w : fijo;
  } else {
    const fijo = lado === "n" ? y + h : y;
    const libre = lado === "n" ? fijo : 1 - fijo;
    h = limitar(lado === "n" ? h - delta : h + delta, min.h, libre);
    if (k !== null) {
      const v = tamanoVisible(e);
      const f = v.ancho / (v.alto * k);
      const centro = x + w / 2;
      const maxW = 2 * Math.min(centro, 1 - centro);
      h = limitar(
        h,
        Math.max(min.h, min.w * f),
        Math.min(libre, Math.max(maxW, w) * f),
      );
      w = h / f;
      x = limitar(centro - w / 2, 0, 1 - w);
    }
    y = lado === "n" ? fijo - h : fijo;
  }
  return { ...e, recorte: { x, y, width: w, height: h } };
}

/**
 * Elige una proporción: el recorte pasa a ser el mayor con esa proporción que
 * cabe en la foto, centrado donde estaba. «Libre» deja el recorte como está.
 */
export function elegirProporcion(
  e: EstadoEditor,
  proporcion: Proporcion,
  vertical = e.vertical,
): EstadoEditor {
  const nuevo = { ...e, proporcion, vertical };
  const k = razon(nuevo);
  if (k === null) return nuevo;
  const v = tamanoVisible(nuevo);
  const f = v.ancho / (v.alto * k);
  let w = 1;
  let h = f;
  if (h > 1) {
    h = 1;
    w = 1 / f;
  }
  const cx = e.recorte.x + e.recorte.width / 2;
  const cy = e.recorte.y + e.recorte.height / 2;
  return {
    ...nuevo,
    recorte: {
      x: limitar(cx - w / 2, 0, 1 - w),
      y: limitar(cy - h / 2, 0, 1 - h),
      width: w,
      height: h,
    },
  };
}

/** Pasa la proporción de horizontal a vertical o al revés. */
export function alternarVertical(e: EstadoEditor): EstadoEditor {
  return elegirProporcion(e, e.proporcion, !e.vertical);
}

/**
 * Gira la foto 90° como se ve: a la izquierda (`"izquierda"`, tecla `R`) o a
 * la derecha (`Mayús` + `R`). El recorte gira con ella.
 */
export function girar(
  e: EstadoEditor,
  sentido: "izquierda" | "derecha",
): EstadoEditor {
  const izquierda = sentido === "izquierda";
  // La receta gira y luego voltea: con la foto volteada, girar a la izquierda
  // lo que se ve es girar a la derecha antes del volteo.
  const horario = izquierda === e.flip_horizontal;
  const rotate = ((e.rotate + (horario ? 90 : 270)) %
    360) as EstadoEditor["rotate"];
  const r = e.recorte;
  const recorte = izquierda
    ? { x: r.y, y: 1 - r.x - r.width, width: r.height, height: r.width }
    : { x: 1 - r.y - r.height, y: r.x, width: r.height, height: r.width };
  // Una proporción fija gira con el recorte; «Original» ya sigue a la foto.
  const fija = !["libre", "original", "cuadrado"].includes(e.proporcion);
  const vertical = fija ? !e.vertical : e.vertical;
  return { ...e, rotate, recorte, vertical };
}

/** Refleja la foto en horizontal, como en un espejo. */
export function voltear(e: EstadoEditor): EstadoEditor {
  const r = e.recorte;
  return {
    ...e,
    flip_horizontal: !e.flip_horizontal,
    recorte: { ...r, x: 1 - r.x - r.width },
  };
}

/** Vuelve a la foto sin editar: sin giro, sin volteo y entera. */
export function restablecer(e: EstadoEditor): EstadoEditor {
  return {
    ...e,
    rotate: 0,
    flip_horizontal: false,
    recorte: { ...ENTERA },
    proporcion: "libre",
    vertical: false,
  };
}

/** Teclas del editor. Devuelve el estado nuevo, o null si la tecla no es suya. */
export function tecla(
  e: EstadoEditor,
  key: string,
  mayus: boolean,
): EstadoEditor | null {
  const paso = mayus ? PASO_LARGO : PASO;
  switch (key) {
    case "ArrowLeft":
      return mover(e, -paso, 0);
    case "ArrowRight":
      return mover(e, paso, 0);
    case "ArrowUp":
      return mover(e, 0, -paso);
    case "ArrowDown":
      return mover(e, 0, paso);
    case "r":
    case "R":
      return girar(e, mayus ? "derecha" : "izquierda");
    default:
      return null;
  }
}

const entera = (r: Recorte) =>
  r.x < EPSILON &&
  r.y < EPSILON &&
  r.width > 1 - EPSILON &&
  r.height > 1 - EPSILON;

/** Geometría para guardar en la receta. Sin recorte si se ve la foto entera. */
export function geometria(e: EstadoEditor): Geometria {
  return {
    rotate: e.rotate,
    flip_horizontal: e.flip_horizontal,
    crop: entera(e.recorte) ? null : { ...e.recorte },
  };
}

const iguales = (a: Recorte | null, b: Recorte | null) => {
  const ra = a ?? ENTERA;
  const rb = b ?? ENTERA;
  return (
    Math.abs(ra.x - rb.x) < EPSILON &&
    Math.abs(ra.y - rb.y) < EPSILON &&
    Math.abs(ra.width - rb.width) < EPSILON &&
    Math.abs(ra.height - rb.height) < EPSILON
  );
};

/** Si la geometría es distinta de la de entrada (para «¿Descartar los cambios?»). */
export function hayCambios(e: EstadoEditor): boolean {
  const g = geometria(e);
  return (
    g.rotate !== e.base.rotate ||
    g.flip_horizontal !== e.base.flip_horizontal ||
    !iguales(g.crop, e.base.crop)
  );
}

/** Si la geometría no cambia nada de la foto (no hay nada que guardar como copia). */
export function esIdentidad(e: EstadoEditor): boolean {
  const g = geometria(e);
  return g.rotate === 0 && !g.flip_horizontal && g.crop === null;
}
