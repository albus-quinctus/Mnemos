import { describe, expect, it } from "vitest";
import type { FotoCuadricula, Mes } from "../biblioteca";
import { disponer, nombreMes, proporcion, visibles, type Bloque, type Opciones } from "./disposicion";

const op: Opciones = { ancho: 1000, altoFila: 200, separacion: 4, altoMes: 48 };

function foto(id: number, month: string, width: number | null, height: number | null): FotoCuadricula {
  return { id, name: `${id}.jpg`, month, date: 0, width, height, thumbnail_failed: false };
}

function filas(bloques: Bloque[]) {
  return bloques.filter((b) => b.tipo === "fila");
}

describe("disponer", () => {
  it("llena cada fila hasta el borde sin recortar las proporciones", () => {
    const fotos = Array.from({ length: 10 }, (_, i) => foto(i, "2026-10", 3000, 2000));
    const { bloques } = disponer([{ month: "2026-10", photos: 10 }], fotos, op);
    const [primera] = filas(bloques);
    const ultima = primera.celdas.at(-1)!;
    expect(ultima.x + ultima.ancho).toBeCloseTo(op.ancho);
    for (const celda of primera.celdas.slice(0, -1)) {
      expect(celda.ancho / primera.alto).toBeCloseTo(1.5);
    }
    // Filas de altura parecida a la objetivo.
    expect(primera.alto).toBeGreaterThan(op.altoFila * 0.7);
    expect(primera.alto).toBeLessThan(op.altoFila * 1.5);
  });

  it("no estira la última fila de cada mes", () => {
    const fotos = [foto(1, "2026-10", 1000, 1000), foto(2, "2026-09", 1000, 1000)];
    const meses: Mes[] = [
      { month: "2026-10", photos: 1 },
      { month: "2026-09", photos: 1 },
    ];
    const { bloques } = disponer(meses, fotos, op);
    expect(bloques.map((b) => b.tipo)).toEqual(["mes", "fila", "mes", "fila"]);
    for (const fila of filas(bloques)) {
      expect(fila.alto).toBe(op.altoFila);
      expect(fila.celdas[0].ancho).toBe(op.altoFila);
    }
  });

  it("coloca los bloques uno debajo de otro y suma el alto total", () => {
    const fotos = Array.from({ length: 30 }, (_, i) => foto(i, i < 20 ? "2026-10" : "2026-09", 4000, 3000));
    const meses: Mes[] = [
      { month: "2026-10", photos: 20 },
      { month: "2026-09", photos: 10 },
    ];
    const { bloques, alto } = disponer(meses, fotos, op);
    for (let i = 1; i < bloques.length; i++) {
      expect(bloques[i].y).toBeGreaterThanOrEqual(bloques[i - 1].y + bloques[i - 1].alto);
    }
    const ultimo = bloques.at(-1)!;
    expect(alto).toBeCloseTo(ultimo.y + ultimo.alto + op.separacion);
    // Cada foto aparece una sola vez y en orden.
    const indices = filas(bloques).flatMap((f) => f.celdas.map((c) => c.indice));
    expect(indices).toEqual(fotos.map((_, i) => i));
  });

  it("reserva el alto de las fotos que aún no se han cargado", () => {
    const meses: Mes[] = [
      { month: "2026-10", photos: 100 },
      { month: "2026-09", photos: 50 },
    ];
    const cargadas = Array.from({ length: 30 }, (_, i) => foto(i, "2026-10", 3000, 2000));
    const { bloques, alto } = disponer(meses, cargadas, op);
    const pendientes = bloques.filter((b) => b.tipo === "pendiente");
    expect(pendientes.map((b) => [b.mes, b.fotos])).toEqual([
      ["2026-10", 70],
      ["2026-09", 50],
    ]);
    // Sin ninguna foto cargada la barra ya refleja toda la biblioteca.
    const vacia = disponer(meses, [], op);
    expect(vacia.alto).toBeGreaterThan(alto * 0.8);
  });

  it("acota las panorámicas y las fotos muy estrechas", () => {
    expect(proporcion({ width: 10000, height: 1000 })).toBe(3);
    expect(proporcion({ width: 100, height: 1000 })).toBe(0.5);
    expect(proporcion({ width: null, height: null })).toBe(1);
    const { bloques } = disponer([{ month: "2026-10", photos: 1 }], [foto(1, "2026-10", 50000, 1000)], {
      ...op,
      ancho: 300,
    });
    const [fila] = filas(bloques);
    expect(fila.celdas[0].ancho).toBeLessThanOrEqual(300 + 1e-6);
  });

  it("10 000 fotos se disponen deprisa", () => {
    const fotos = Array.from({ length: 10_000 }, (_, i) =>
      foto(i, `20${String(10 + Math.floor(i / 500)).padStart(2, "0")}-01`, 3000 + (i % 7) * 300, 2000),
    );
    const meses: Mes[] = [];
    for (const f of fotos) {
      const ultimo = meses.at(-1);
      if (ultimo?.month === f.month) ultimo.photos++;
      else meses.push({ month: f.month, photos: 1 });
    }
    const inicio = performance.now();
    const { bloques } = disponer(meses, fotos, op);
    expect(performance.now() - inicio).toBeLessThan(500);
    expect(filas(bloques).flatMap((f) => f.celdas)).toHaveLength(10_000);
  });
});

describe("visibles", () => {
  it("devuelve solo los bloques que tocan la ventana", () => {
    const bloques: Bloque[] = Array.from({ length: 100 }, (_, i) => ({
      tipo: "mes",
      mes: String(i),
      y: i * 100,
      alto: 100,
    }));
    const vistos = visibles(bloques, 1050, 1250);
    expect(vistos.map((b) => b.y)).toEqual([1000, 1100, 1200]);
    expect(visibles(bloques, 50_000, 60_000)).toEqual([]);
  });
});

describe("nombreMes", () => {
  it("escribe el mes en español con mayúscula", () => {
    expect(nombreMes("2026-10")).toBe("Octubre de 2026");
    expect(nombreMes("2024-05")).toBe("Mayo de 2024");
  });
});
