import { beforeEach, describe, expect, it, vi } from "vitest";

// Núcleo simulado: limpia como `set_day_title` (saltos de línea a espacios,
// recorta, 100 caracteres) y vacío borra.
const guardados = new Map<string, string>([["2024-10-12", "Ruta del Cares"]]);
let fallar = false;

vi.mock("../biblioteca", () => ({
  titulosDia: vi.fn(async () => [...guardados].map(([day, title]) => ({ day, title }))),
  ponerTituloDia: vi.fn(async (dia: string, titulo: string) => {
    if (fallar) throw new Error("disco desconectado");
    const limpio = titulo.replace(/[\r\n\t]/g, " ").trim().slice(0, 100);
    if (!limpio) {
      guardados.delete(dia);
      return null;
    }
    guardados.set(dia, limpio);
    return limpio;
  }),
}));

import * as biblioteca from "../biblioteca";
import { cargarTitulos, guardarTitulo, olvidarTitulos, titulos } from "./titulos.svelte";

describe("títulos de día", () => {
  beforeEach(() => {
    fallar = false;
    guardados.clear();
    guardados.set("2024-10-12", "Ruta del Cares");
    olvidarTitulos();
    vi.clearAllMocks();
  });

  it("se leen una sola vez para todas las cuadrículas", async () => {
    await Promise.all([cargarTitulos(), cargarTitulos()]);
    await cargarTitulos();
    expect(biblioteca.titulosDia).toHaveBeenCalledTimes(1);
    expect(titulos.get("2024-10-12")).toBe("Ruta del Cares");
  });

  it("al cambiar de biblioteca se vuelven a leer", async () => {
    await cargarTitulos();
    olvidarTitulos();
    expect(titulos.size).toBe(0);
    await cargarTitulos();
    expect(biblioteca.titulosDia).toHaveBeenCalledTimes(2);
  });

  it("guarda el texto tal como lo deja el núcleo", async () => {
    await guardarTitulo("2024-10-11", "  Picos\nde Europa 🏔️ «sí» ");
    expect(titulos.get("2024-10-11")).toBe("Picos de Europa 🏔️ «sí»");
  });

  it("un título vacío lo borra", async () => {
    await cargarTitulos();
    await guardarTitulo("2024-10-12", "   ");
    expect(titulos.has("2024-10-12")).toBe(false);
  });

  it("si falla, el título anterior se conserva", async () => {
    await cargarTitulos();
    fallar = true;
    await expect(guardarTitulo("2024-10-12", "Otro")).rejects.toThrow("disco desconectado");
    expect(titulos.get("2024-10-12")).toBe("Ruta del Cares");
  });
});
