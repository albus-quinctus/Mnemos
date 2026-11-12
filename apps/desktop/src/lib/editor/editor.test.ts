import { describe, expect, it } from "vitest";
import {
  alternarVertical,
  arrastrarEsquina,
  arrastrarLado,
  crearEstado,
  elegirProporcion,
  esIdentidad,
  geometria,
  girar,
  hayCambios,
  mover,
  restablecer,
  tamanoRecorte,
  tamanoVisible,
  tecla,
  voltear,
  type EstadoEditor,
} from "./editor";

// Foto de 600 × 400 ya girada según el EXIF.
const foto = () => crearEstado(600, 400);

const conRecorte = (
  e: EstadoEditor,
  x: number,
  y: number,
  w: number,
  h: number,
) => ({
  ...e,
  recorte: { x, y, width: w, height: h },
});

function cerca(
  r: { x: number; y: number; width: number; height: number },
  esperado: number[],
) {
  const [x, y, w, h] = esperado;
  expect(r.x).toBeCloseTo(x, 6);
  expect(r.y).toBeCloseTo(y, 6);
  expect(r.width).toBeCloseTo(w, 6);
  expect(r.height).toBeCloseTo(h, 6);
}

/** Proporción en píxeles del recorte. */
const proporcionPx = (e: EstadoEditor) => {
  const v = tamanoVisible(e);
  return (e.recorte.width * v.ancho) / (e.recorte.height * v.alto);
};

describe("al entrar", () => {
  it("el marco cubre la foto entera y no hay nada que guardar", () => {
    const e = foto();
    cerca(e.recorte, [0, 0, 1, 1]);
    expect(geometria(e)).toEqual({
      rotate: 0,
      flip_horizontal: false,
      crop: null,
    });
    expect(hayCambios(e)).toBe(false);
    expect(esIdentidad(e)).toBe(true);
  });

  it("parte de la versión que se ve", () => {
    const base = {
      rotate: 90 as const,
      flip_horizontal: true,
      crop: { x: 0.1, y: 0.2, width: 0.5, height: 0.6 },
    };
    const e = crearEstado(600, 400, base);
    expect(geometria(e)).toEqual(base);
    expect(hayCambios(e)).toBe(false);
    expect(tamanoVisible(e)).toEqual({ ancho: 400, alto: 600 });
  });
});

describe("arrastrar esquinas", () => {
  it("la esquina opuesta queda fija", () => {
    const e = arrastrarEsquina(foto(), "se", -0.3, -0.2);
    cerca(e.recorte, [0, 0, 0.7, 0.8]);
    const f = arrastrarEsquina(e, "no", 0.1, 0.1);
    cerca(f.recorte, [0.1, 0.1, 0.6, 0.7]);
  });

  it("nunca sale de la foto", () => {
    const e = conRecorte(foto(), 0.2, 0.2, 0.5, 0.5);
    cerca(arrastrarEsquina(e, "se", 0.9, 0.9).recorte, [0.2, 0.2, 0.8, 0.8]);
    cerca(arrastrarEsquina(e, "no", -0.9, -0.9).recorte, [0, 0, 0.7, 0.7]);
  });

  it("mide al menos 32 × 32 píxeles del original", () => {
    const e = arrastrarEsquina(foto(), "se", -2, -2);
    expect(tamanoRecorte(e)).toEqual({ ancho: 32, alto: 32 });
    // Y no se da la vuelta al pasar de la esquina opuesta.
    cerca(e.recorte, [0, 0, 32 / 600, 32 / 400]);
  });

  it("con proporción, la mantiene y cabe en la foto", () => {
    let e = elegirProporcion(foto(), "4:3");
    expect(proporcionPx(e)).toBeCloseTo(4 / 3, 6);
    e = arrastrarEsquina(e, "se", -0.3, -0.05);
    expect(proporcionPx(e)).toBeCloseTo(4 / 3, 6);
    e = arrastrarEsquina(e, "no", -2, -2);
    expect(proporcionPx(e)).toBeCloseTo(4 / 3, 6);
    expect(e.recorte.x).toBeGreaterThanOrEqual(0);
    expect(e.recorte.y).toBeGreaterThanOrEqual(0);
    expect(e.recorte.x + e.recorte.width).toBeLessThanOrEqual(1 + 1e-9);
    expect(e.recorte.y + e.recorte.height).toBeLessThanOrEqual(1 + 1e-9);
  });
});

describe("arrastrar lados", () => {
  it("mueve solo ese lado", () => {
    const e = arrastrarLado(foto(), "e", -0.25);
    cerca(e.recorte, [0, 0, 0.75, 1]);
    cerca(arrastrarLado(e, "n", 0.1).recorte, [0, 0.1, 0.75, 0.9]);
    cerca(arrastrarLado(e, "o", -0.5).recorte, [0, 0, 0.75, 1]);
  });

  it("con proporción, el otro lado se ajusta centrado", () => {
    let e = elegirProporcion(foto(), "cuadrado");
    e = arrastrarLado(e, "e", -0.2);
    expect(proporcionPx(e)).toBeCloseTo(1, 6);
    const centro = e.recorte.y + e.recorte.height / 2;
    expect(centro).toBeCloseTo(0.5, 6);
  });
});

describe("mover", () => {
  it("conserva el tamaño y no sale de la foto", () => {
    const e = conRecorte(foto(), 0.2, 0.2, 0.5, 0.5);
    cerca(mover(e, 0.1, -0.1).recorte, [0.3, 0.1, 0.5, 0.5]);
    cerca(mover(e, 5, -5).recorte, [0.5, 0, 0.5, 0.5]);
  });

  it("con las flechas, 1 % y 10 % con Mayús", () => {
    const e = conRecorte(foto(), 0.2, 0.2, 0.5, 0.5);
    cerca(tecla(e, "ArrowRight", false)!.recorte, [0.21, 0.2, 0.5, 0.5]);
    cerca(tecla(e, "ArrowUp", true)!.recorte, [0.2, 0.1, 0.5, 0.5]);
    expect(tecla(e, "a", false)).toBeNull();
  });
});

describe("proporciones", () => {
  it("elige el mayor recorte que cabe, centrado donde estaba", () => {
    const e = elegirProporcion(foto(), "cuadrado");
    cerca(e.recorte, [1 / 6, 0, 2 / 3, 1]);
    expect(tamanoRecorte(e)).toEqual({ ancho: 400, alto: 400 });
  });

  it("«Original» es la de la foto ya girada", () => {
    const e = elegirProporcion(conRecorte(foto(), 0, 0, 0.5, 0.5), "original");
    cerca(e.recorte, [0, 0, 1, 1]);
    const g = elegirProporcion(girar(foto(), "izquierda"), "original");
    expect(proporcionPx(g)).toBeCloseTo(400 / 600, 6);
  });

  it("vertical da la vuelta a la proporción", () => {
    const e = alternarVertical(elegirProporcion(foto(), "16:9"));
    expect(proporcionPx(e)).toBeCloseTo(9 / 16, 6);
    expect(e.recorte.height).toBeCloseTo(1, 6);
  });

  it("«Libre» deja el recorte como está", () => {
    const e = conRecorte(foto(), 0.1, 0.1, 0.3, 0.3);
    cerca(elegirProporcion(e, "libre").recorte, [0.1, 0.1, 0.3, 0.3]);
  });
});

describe("girar y voltear", () => {
  it("el recorte gira con la foto", () => {
    // Cuarto de arriba a la izquierda: a la izquierda pasa abajo a la izquierda.
    const e = conRecorte(foto(), 0, 0, 0.5, 0.5);
    const izq = girar(e, "izquierda");
    expect(izq.rotate).toBe(270);
    cerca(izq.recorte, [0, 0.5, 0.5, 0.5]);
    const der = girar(e, "derecha");
    expect(der.rotate).toBe(90);
    cerca(der.recorte, [0.5, 0, 0.5, 0.5]);
  });

  it("cuatro giros o girar y volver dejan la foto como estaba", () => {
    const e = conRecorte(foto(), 0.1, 0.2, 0.3, 0.4);
    let g = e;
    for (let i = 0; i < 4; i++) g = girar(g, "izquierda");
    expect(g.rotate).toBe(0);
    cerca(g.recorte, [0.1, 0.2, 0.3, 0.4]);
    const ida = girar(girar(e, "izquierda"), "derecha");
    cerca(ida.recorte, [0.1, 0.2, 0.3, 0.4]);
    expect(hayCambios(ida)).toBe(true); // el recorte sí es un cambio
    expect(hayCambios(girar(girar(foto(), "izquierda"), "derecha"))).toBe(
      false,
    );
  });

  it("con la foto volteada, girar a la izquierda suma 90° antes del volteo", () => {
    const e = voltear(foto());
    expect(girar(e, "izquierda").rotate).toBe(90);
    expect(girar(e, "derecha").rotate).toBe(270);
  });

  it("una proporción fija pasa a vertical al girar", () => {
    const e = girar(elegirProporcion(foto(), "3:2"), "izquierda");
    expect(e.vertical).toBe(true);
    expect(proporcionPx(e)).toBeCloseTo(2 / 3, 6);
  });

  it("R gira a la izquierda y Mayús + R a la derecha", () => {
    expect(tecla(foto(), "r", false)!.rotate).toBe(270);
    expect(tecla(foto(), "R", true)!.rotate).toBe(90);
  });

  it("voltear refleja el recorte y dos veces lo deja igual", () => {
    const e = conRecorte(foto(), 0.1, 0.2, 0.3, 0.4);
    const v = voltear(e);
    expect(v.flip_horizontal).toBe(true);
    cerca(v.recorte, [0.6, 0.2, 0.3, 0.4]);
    cerca(voltear(v).recorte, [0.1, 0.2, 0.3, 0.4]);
  });
});

describe("cambios sin guardar", () => {
  it("cualquier cambio cuenta y restablecer vuelve a la foto sin editar", () => {
    const base = { rotate: 90 as const, flip_horizontal: false, crop: null };
    let e = crearEstado(600, 400, base);
    e = voltear(e);
    expect(hayCambios(e)).toBe(true);
    e = restablecer(e);
    expect(geometria(e)).toEqual({
      rotate: 0,
      flip_horizontal: false,
      crop: null,
    });
    // Restablecer cambia la versión de entrada, que estaba girada.
    expect(hayCambios(e)).toBe(true);
    expect(esIdentidad(e)).toBe(true);
  });
});
