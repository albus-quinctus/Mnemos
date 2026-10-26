import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { busqueda, escribir, limpiarBusqueda } from "./busqueda.svelte";

describe("campo de búsqueda", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => {
    limpiarBusqueda();
    vi.useRealTimers();
  });

  it("busca 200 ms después de la última tecla", () => {
    escribir("p");
    vi.advanceTimersByTime(150);
    escribir("pl");
    vi.advanceTimersByTime(150);
    expect(busqueda.texto).toBe("pl");
    expect(busqueda.consulta).toBe("");
    vi.advanceTimersByTime(50);
    expect(busqueda.consulta).toBe("pl");
  });

  it("los espacios solos no buscan", () => {
    escribir("   ");
    vi.advanceTimersByTime(200);
    expect(busqueda.consulta).toBe("");
  });

  it("limpiar vuelve a la biblioteca entera y cancela lo pendiente", () => {
    escribir("playa");
    vi.advanceTimersByTime(200);
    escribir("playa 2024");
    limpiarBusqueda();
    vi.advanceTimersByTime(500);
    expect(busqueda.texto).toBe("");
    expect(busqueda.consulta).toBe("");
  });
});
