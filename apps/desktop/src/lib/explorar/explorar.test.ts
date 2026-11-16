import { describe, expect, it } from "vitest";
import {
  coleccionDe,
  esVistaExplorar,
  haceAnios,
  importadasEl,
  textoElementos,
  textoRango,
  ultimaImportacion,
} from "./explorar";

/** Segundos Unix de una medianoche local. */
const medianoche = (a: number, m: number, d: number) => new Date(a, m - 1, d).getTime() / 1000;

describe("textos de Explorar", () => {
  it("dice hace cuántos años", () => {
    expect(haceAnios(1)).toBe("Hace 1 año");
    expect(haceAnios(7)).toBe("Hace 7 años");
  });

  it("cuenta elementos", () => {
    expect(textoElementos(1)).toBe("1 elemento");
    expect(textoElementos(42)).toBe("42 elementos");
  });

  it("escribe el rango de un recuerdo", () => {
    expect(textoRango(medianoche(2025, 11, 12), medianoche(2025, 11, 19))).toBe("12–18 nov 2025");
    expect(textoRango(medianoche(2025, 10, 28), medianoche(2025, 11, 4))).toBe(
      "28 oct – 3 nov 2025",
    );
    expect(textoRango(medianoche(2024, 12, 29), medianoche(2025, 1, 5))).toBe(
      "29 dic 2024 – 4 ene 2025",
    );
  });

  it("dice cuándo fue la última importación", () => {
    const hoy = new Date(2026, 10, 16);
    expect(ultimaImportacion(medianoche(2026, 11, 14) + 3600, hoy)).toBe(
      "Última importación: 14 nov",
    );
    expect(ultimaImportacion(medianoche(2025, 3, 2), hoy)).toBe("Última importación: 2 mar 2025");
  });

  it("titula los días de Añadidas recientemente", () => {
    const hoy = new Date(2026, 10, 16);
    expect(importadasEl("2026-11-14", hoy)).toBe("Importadas el 14 nov");
    expect(importadasEl("2025-01-02", hoy)).toBe("Importadas el 2 ene 2025");
  });

  it("reconoce las vistas de Explorar", () => {
    expect(coleccionDe({ coleccion: "videos" })).toBe("videos");
    expect(coleccionDe({ album: 3 })).toBeNull();
    expect(coleccionDe("fotos")).toBeNull();
    expect(esVistaExplorar({ rango: { desde: 1, hasta: 2 } })).toBe(true);
    expect(esVistaExplorar({ coleccion: "raw" })).toBe(true);
    expect(esVistaExplorar({ etiqueta: 1 })).toBe(false);
  });
});
