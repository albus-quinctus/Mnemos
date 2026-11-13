import { beforeEach, describe, expect, it, vi } from "vitest";

// Núcleo simulado: la ventana está o no en pantalla completa.
let completa = false;
let sinTauri = false;
let temaGuardado: string | null = null;

vi.mock("../biblioteca", async (original) => ({
  ...(await original<typeof import("../biblioteca")>()),
  estaEnPantallaCompleta: vi.fn(async () => {
    if (sinTauri) throw new Error("sin Tauri");
    return completa;
  }),
  alternarPantallaCompleta: vi.fn(async () => {
    if (sinTauri) throw new Error("sin Tauri");
    completa = !completa;
    return completa;
  }),
  ponerTema: vi.fn(async (tema: string) => {
    temaGuardado = tema;
  }),
}));

// Sin DOM en las pruebas: un `<html>` con su `dataset`.
const html = { dataset: {} as DOMStringMap };
vi.stubGlobal("document", { documentElement: html });

import {
  abrirAjustes,
  ajustes,
  cambiarPantallaCompleta,
  cambiarTema,
  cerrarAjustes,
} from "./ajustes.svelte";

beforeEach(() => {
  completa = false;
  sinTauri = false;
  temaGuardado = null;
  html.dataset = {};
  cerrarAjustes();
  ajustes.pantallaCompleta = false;
});

describe("ventana de Ajustes", () => {
  it("al abrirse muestra el tema de ahora y el estado real de la ventana", async () => {
    html.dataset.tema = "oscuro";
    completa = true;
    await abrirAjustes();
    expect(ajustes.abierta).toBe(true);
    expect(ajustes.tema).toBe("oscuro");
    expect(ajustes.pantallaCompleta).toBe(true);
    cerrarAjustes();
    expect(ajustes.abierta).toBe(false);
  });

  it("sin data-tema, el tema es Sistema", async () => {
    await abrirAjustes();
    expect(ajustes.tema).toBe("sistema");
  });

  it("cambiar el tema lo aplica en <html> y lo guarda", async () => {
    await abrirAjustes();
    await cambiarTema("claro");
    expect(ajustes.tema).toBe("claro");
    expect(html.dataset.tema).toBe("claro");
    expect(temaGuardado).toBe("claro");
    await cambiarTema("sistema");
    expect(html.dataset.tema).toBeUndefined();
    expect(temaGuardado).toBe("sistema");
  });

  it("el interruptor alterna la pantalla completa", async () => {
    await abrirAjustes();
    await cambiarPantallaCompleta();
    expect(ajustes.pantallaCompleta).toBe(true);
    await cambiarPantallaCompleta();
    expect(ajustes.pantallaCompleta).toBe(false);
  });

  it("fuera de Tauri se abre igual y el interruptor no cambia", async () => {
    sinTauri = true;
    await abrirAjustes();
    expect(ajustes.abierta).toBe(true);
    await cambiarPantallaCompleta();
    expect(ajustes.pantallaCompleta).toBe(false);
  });
});
