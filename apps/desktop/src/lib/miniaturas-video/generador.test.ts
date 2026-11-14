import { describe, expect, it } from "vitest";
import { crearGenerador, medidasMiniatura, momentoFotograma } from "./generador";

describe("medidasMiniatura", () => {
  it("deja 360 px de alto y respeta la proporción", () => {
    expect(medidasMiniatura(1920, 1080)).toEqual({ ancho: 640, alto: 360 });
    expect(medidasMiniatura(1080, 1920)).toEqual({ ancho: 203, alto: 360 });
  });
  it("no amplía vídeos pequeños y limita los muy anchos", () => {
    expect(medidasMiniatura(320, 180)).toEqual({ ancho: 320, alto: 180 });
    expect(medidasMiniatura(6000, 400)).toEqual({ ancho: 1080, alto: 72 });
  });
  it("sin imagen, cero", () => {
    expect(medidasMiniatura(0, 0)).toEqual({ ancho: 0, alto: 0 });
  });
});

describe("momentoFotograma", () => {
  it("el segundo 1, o la mitad si el vídeo es corto", () => {
    expect(momentoFotograma(42)).toBe(1);
    expect(momentoFotograma(1)).toBe(0.5);
    expect(momentoFotograma(NaN)).toBe(0);
    expect(momentoFotograma(Infinity)).toBe(0);
  });
});

describe("crearGenerador", () => {
  function biblioteca(ids: number[], rotos: number[] = []) {
    const sinMiniatura = new Set(ids);
    const guardadas: number[] = [];
    const fallidas: number[] = [];
    let pedidas = 0;
    const gen = crearGenerador({
      pendientes: async (limite) => {
        pedidas++;
        return [...sinMiniatura].slice(0, limite);
      },
      sacar: async (id) => {
        if (rotos.includes(id)) throw new Error("códec desconocido");
        return new Uint8Array([0xff, 0xd8, id]);
      },
      guardar: async (id) => {
        sinMiniatura.delete(id);
        guardadas.push(id);
      },
      fallida: async (id) => {
        fallidas.push(id);
      },
    });
    return { gen, guardadas, fallidas, pedidas: () => pedidas };
  }

  it("hace todas las que faltan, en lotes", async () => {
    const ids = Array.from({ length: 45 }, (_, i) => i + 1);
    const b = biblioteca(ids);
    await b.gen.arrancar();
    expect(b.guardadas).toEqual(ids);
    expect(b.fallidas).toEqual([]);
  });

  it("avisa de las que fallan y no se atasca con ellas", async () => {
    const rotos = Array.from({ length: 25 }, (_, i) => i + 1);
    const b = biblioteca([...rotos, 100], rotos);
    await b.gen.arrancar();
    expect(b.guardadas).toEqual([100]);
    expect(b.fallidas).toEqual(rotos);
    // Otra vuelta no las repite.
    await b.gen.arrancar();
    expect(b.fallidas).toEqual(rotos);
  });

  it("si se arranca mientras trabaja, da otra vuelta al terminar", async () => {
    const b = biblioteca([1, 2]);
    const primera = b.gen.arrancar();
    void b.gen.arrancar();
    await primera;
    expect(b.guardadas).toEqual([1, 2]);
    expect(b.pedidas()).toBeGreaterThanOrEqual(3);
  });

  it("parado no hace nada más", async () => {
    const b = biblioteca([1, 2, 3]);
    b.gen.parar();
    await b.gen.arrancar();
    expect(b.guardadas).toEqual([]);
  });
});
