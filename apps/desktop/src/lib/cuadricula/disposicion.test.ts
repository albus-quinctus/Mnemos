import { describe, expect, it } from "vitest";
import type { FotoCuadricula, Mes } from "../biblioteca";
import {
  diaDe,
  disponer,
  mesEn,
  nombreDia,
  nombreMes,
  proporcion,
  visibles,
  type Bloque,
  type Opciones,
} from "./disposicion";

const op: Opciones = { ancho: 1000, altoFila: 180, separacion: 4, separacionDias: 16, altoTitulo: 40 };

// Mediodía local, para que el día no dependa de la zona horaria.
function fecha(dia: string): number {
  const [a, m, d] = dia.split("-").map(Number);
  return new Date(a, m - 1, d, 12).getTime() / 1000;
}

function foto(id: number, dia: string, width: number | null = 3000, height: number | null = 2000): FotoCuadricula {
  return { id, name: `${id}.jpg`, month: dia.slice(0, 7), date: fecha(dia), width, height, thumbnail_failed: false };
}

function mesesDe(fotos: FotoCuadricula[]): Mes[] {
  const meses: Mes[] = [];
  for (const f of fotos) {
    const ultimo = meses.at(-1);
    if (ultimo?.month === f.month) ultimo.photos++;
    else meses.push({ month: f.month, photos: 1 });
  }
  return meses;
}

type Fila = Extract<Bloque, { tipo: "fila" }>;
const filas = (bloques: Bloque[]) => bloques.filter((b): b is Fila => b.tipo === "fila");

describe("disponer", () => {
  it("llena las filas de un día hasta el borde sin recortar las proporciones", () => {
    const fotos = Array.from({ length: 12 }, (_, i) => foto(i, "2026-10-20"));
    const { bloques } = disponer(mesesDe(fotos), fotos, op);
    const [primera] = filas(bloques);
    const ultima = primera.celdas.at(-1)!;
    expect(ultima.x + ultima.ancho).toBeCloseTo(op.ancho);
    const altoFotos = primera.alto - primera.altoTitulo;
    for (const celda of primera.celdas.slice(0, -1)) expect(celda.ancho / altoFotos).toBeCloseTo(1.5);
    expect(altoFotos).toBeGreaterThan(op.altoFila * 0.7);
    expect(altoFotos).toBeLessThan(op.altoFila * 1.5);
  });

  it("pone un título encima de cada día", () => {
    const fotos = [...Array.from({ length: 12 }, (_, i) => foto(i, "2026-10-20")), foto(99, "2026-10-19")];
    const { bloques } = disponer(mesesDe(fotos), fotos, op);
    const titulos = filas(bloques).flatMap((f) => f.titulos.map((t) => t.dia));
    expect(titulos).toEqual(["2026-10-20", "2026-10-19"]);
    // Solo la primera fila de cada día deja sitio para el título.
    expect(filas(bloques).map((f) => f.altoTitulo > 0)).toEqual([true, false, false, true]);
  });

  it("los días con pocas fotos comparten fila, separados por un hueco mayor", () => {
    const cuadrada = (id: number, dia: string) => foto(id, dia, 1000, 1000);
    const fotos = [cuadrada(1, "2026-10-20"), cuadrada(2, "2026-10-20"), cuadrada(3, "2026-10-18"), cuadrada(4, "2026-09-30")];
    const { bloques } = disponer(mesesDe(fotos), fotos, op);
    expect(filas(bloques)).toHaveLength(1);
    const [fila] = filas(bloques);
    expect(fila.titulos.map((t) => t.dia)).toEqual(["2026-10-20", "2026-10-18", "2026-09-30"]);
    const [, segunda, tercera] = fila.celdas;
    expect(tercera.x - (segunda.x + segunda.ancho)).toBeCloseTo(op.separacionDias);
    expect(fila.titulos[1].x).toBe(tercera.x);
    // Ninguna celda se sale de la fila.
    for (const c of fila.celdas) expect(c.x + c.ancho).toBeLessThanOrEqual(op.ancho + 1e-6);
  });

  it("un día de una foto vertical deja sitio a su título antes del siguiente", () => {
    // 180 px de alto con proporción 2:3 son 120 px de ancho, menos que el título.
    const fotos = [foto(1, "2025-11-30", 2000, 3000), foto(2, "2025-10-25", 2000, 3000)];
    const [fila] = filas(disponer(mesesDe(fotos), fotos, { ...op, anchoMinDia: 140 }).bloques);
    expect(fila.titulos.map((t) => t.x)).toEqual([0, 140 + op.separacionDias]);
    expect(fila.celdas[0].ancho).toBe(120);
  });

  it("un día que no cabe en lo que queda de fila baja a la siguiente", () => {
    const fotos = [
      ...Array.from({ length: 3 }, (_, i) => foto(i, "2026-10-20")),
      ...Array.from({ length: 3 }, (_, i) => foto(10 + i, "2026-10-19")),
    ];
    const { bloques } = disponer(mesesDe(fotos), fotos, op);
    expect(filas(bloques).map((f) => f.titulos.map((t) => t.dia))).toEqual([["2026-10-20"], ["2026-10-19"]]);
  });

  it("cada foto aparece una vez, en orden, y los bloques no se pisan", () => {
    const dias = ["2026-10-20", "2026-10-18", "2026-10-02", "2026-09-30", "2026-09-01"];
    const fotos = Array.from({ length: 60 }, (_, i) => foto(i, dias[Math.floor(i / 12)], 3000 + (i % 5) * 400, 2000));
    const { bloques, alto } = disponer(mesesDe(fotos), fotos, op);
    for (let i = 1; i < bloques.length; i++) {
      expect(bloques[i].y).toBeGreaterThanOrEqual(bloques[i - 1].y + bloques[i - 1].alto);
    }
    const ultimo = bloques.at(-1)!;
    expect(alto).toBeCloseTo(ultimo.y + ultimo.alto + op.separacion);
    expect(filas(bloques).flatMap((f) => f.celdas.map((c) => c.indice))).toEqual(fotos.map((_, i) => i));
  });

  it("reserva el alto de lo no cargado y marca dónde empieza cada mes", () => {
    const meses: Mes[] = [
      { month: "2026-10", photos: 100 },
      { month: "2026-09", photos: 50 },
    ];
    const cargadas = Array.from({ length: 30 }, (_, i) => foto(i, `2026-10-${String(20 - Math.floor(i / 10)).padStart(2, "0")}`));
    const { bloques, marcas, alto } = disponer(meses, cargadas, op);
    expect(bloques.filter((b) => b.tipo === "pendiente").map((b) => [b.mes, b.fotos])).toEqual([
      ["2026-10", 70],
      ["2026-09", 50],
    ]);
    expect(marcas.map((m) => m.mes)).toEqual(["2026-10", "2026-09"]);
    expect(marcas[0].y).toBe(0);
    expect(marcas[1].y).toBeGreaterThan(0);
    expect(mesEn(marcas, marcas[1].y + 1)).toBe("2026-09");
    expect(mesEn(marcas, 10)).toBe("2026-10");
    // Sin ninguna foto cargada la barra ya refleja toda la biblioteca.
    expect(disponer(meses, [], op).alto).toBeGreaterThan(alto * 0.7);
  });

  it("acota las panorámicas y las fotos muy estrechas", () => {
    expect(proporcion({ width: 10000, height: 1000 })).toBe(3);
    expect(proporcion({ width: 100, height: 1000 })).toBe(0.5);
    expect(proporcion({ width: null, height: null })).toBe(1);
    const fotos = [foto(1, "2026-10-20", 50000, 1000)];
    const { bloques } = disponer(mesesDe(fotos), fotos, { ...op, ancho: 300 });
    expect(filas(bloques)[0].celdas[0].ancho).toBeLessThanOrEqual(300 + 1e-6);
  });

  it("10 000 fotos se disponen deprisa", () => {
    const fotos = Array.from({ length: 10_000 }, (_, i) => {
      const d = new Date(2026, 9, 26 - Math.floor(i / 7), 12);
      const dia = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
      return foto(i, dia, 3000 + (i % 7) * 300, 2000);
    });
    const inicio = performance.now();
    const { bloques } = disponer(mesesDe(fotos), fotos, op);
    expect(performance.now() - inicio).toBeLessThan(500);
    expect(filas(bloques).flatMap((f) => f.celdas)).toHaveLength(10_000);
  });
});

describe("visibles", () => {
  it("devuelve solo los bloques que tocan la ventana", () => {
    const bloques: Bloque[] = Array.from({ length: 100 }, (_, i) => ({
      tipo: "pendiente",
      mes: String(i),
      y: i * 100,
      alto: 100,
      fotos: 1,
    }));
    expect(visibles(bloques, 1050, 1250).map((b) => b.y)).toEqual([1000, 1100, 1200]);
    expect(visibles(bloques, 50_000, 60_000)).toEqual([]);
  });
});

describe("nombres", () => {
  it("escribe el mes en español con mayúscula", () => {
    expect(nombreMes("2026-10")).toBe("Octubre de 2026");
  });

  it("escribe el día como Google Fotos, con el año solo si no es el actual", () => {
    const hoy = new Date(2026, 9, 26);
    expect(nombreDia("2026-10-20", hoy)).toBe("Mar, 20 oct");
    expect(nombreDia("2025-10-20", hoy)).toBe("Lun, 20 oct 2025");
  });

  it("el día va en hora local", () => {
    expect(diaDe({ date: fecha("2026-10-20") })).toBe("2026-10-20");
  });
});

// Spec 002, tarea 15: la cuadrícula debe ir fluida con 10 000 fotos. Los
// límites son holgados para no fallar en un CI lento; los tiempos reales se
// anotan en docs/rendimiento.md.
describe("rendimiento con 10 000 fotos", () => {
  const proporciones: [number, number][] = [[4000, 3000], [3000, 4000], [6000, 4000], [1080, 1920], [9000, 3000]];
  const fotos: FotoCuadricula[] = [];
  const inicio = new Date(2026, 9, 26, 12);
  for (let i = 0; i < 10_000; i++) {
    // Entre 1 y 12 fotos por día, hacia atrás.
    const d = new Date(inicio);
    d.setDate(d.getDate() - Math.floor(i / (1 + (i % 12))));
    const dia = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const [w, h] = proporciones[i % proporciones.length];
    fotos.push(foto(i + 1, dia, w, h));
  }
  fotos.sort((a, b) => b.date - a.date || b.id - a.id);
  const meses = mesesDe(fotos);

  it("dispone toda la biblioteca de una vez", () => {
    const t0 = performance.now();
    const { bloques, alto } = disponer(meses, fotos, { ...op, ancho: 1280, altoFila: 150 });
    const tiempo = performance.now() - t0;
    expect(filas(bloques).flatMap((f) => f.celdas)).toHaveLength(10_000);
    expect(alto).toBeGreaterThan(0);
    expect(tiempo).toBeLessThan(250);
  });

  it("encuentra lo visible en cada fotograma sin recorrer toda la lista", () => {
    const { bloques, alto } = disponer(meses, fotos, { ...op, ancho: 1280, altoFila: 150 });
    const veces = 1000;
    const t0 = performance.now();
    for (let i = 0; i < veces; i++) {
      const y = (alto * i) / veces;
      visibles(bloques, y - 1200, y + 800 + 1200);
    }
    const porFotograma = (performance.now() - t0) / veces;
    // Un fotograma a 60 fps dura 16,7 ms.
    expect(porFotograma).toBeLessThan(2);
  });
});
