import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// El núcleo se simula: 10 fotos en el orden de la cuadrícula (la más nueva
// primero), repartidas en dos días. Solo la cuadrícula tendría cargadas
// algunas; los rangos, días y Ctrl + A salen siempre de aquí.
const fotos = Array.from({ length: 10 }, (_, i) => ({
  id: 100 + i,
  date: 1_000 - i,
  dia: i < 6 ? "2024-10-12" : "2024-10-11",
}));

vi.mock("../biblioteca", () => ({
  seleccionarTodo: vi.fn(async () => fotos.map((f) => f.id)),
  seleccionarDia: vi.fn(async (_ambito: unknown, dia: string) =>
    fotos.filter((f) => f.dia === dia).map((f) => f.id),
  ),
  seleccionarRango: vi.fn(
    async (_ambito: unknown, a: { id: number }, b: { id: number }) => {
      const i = fotos.findIndex((f) => f.id === a.id);
      const j = fotos.findIndex((f) => f.id === b.id);
      return fotos
        .slice(Math.min(i, j), Math.max(i, j) + 1)
        .map((f) => f.id);
    },
  ),
}));

import * as biblioteca from "../biblioteca";
import {
  alFallar,
  alternarDia,
  cambiarAmbito,
  clicEnFoto,
  estaSeleccionada,
  haySeleccion,
  limpiarSeleccion,
  seleccion,
  seleccionarHasta,
  seleccionarVista,
  teclaSeleccion,
  todasSeleccionadas,
} from "./seleccion.svelte";

const foto = (i: number) => fotos[i];
const seleccionadas = () => [...seleccion.ids].sort((a, b) => a - b);
// Deja que terminen las promesas de un clic o una tecla.
const esperar = () => new Promise((r) => setTimeout(r, 0));

describe("selección múltiple", () => {
  beforeEach(() => {
    cambiarAmbito({ vista: "fotos" });
    limpiarSeleccion();
    vi.clearAllMocks();
  });
  afterEach(() => alFallar((e) => console.error(e)));

  it("sin selección, un clic abre el visor", () => {
    expect(clicEnFoto(foto(0))).toBe(false);
    expect(haySeleccion()).toBe(false);
  });

  it("Ctrl + clic marca y desmarca sin abrir", () => {
    expect(clicEnFoto(foto(2), { ctrl: true })).toBe(true);
    expect(estaSeleccionada(102)).toBe(true);
    expect(clicEnFoto(foto(2), { ctrl: true })).toBe(true);
    expect(haySeleccion()).toBe(false);
  });

  it("en modo selección, un clic marca o desmarca", () => {
    clicEnFoto(foto(0), { ctrl: true });
    expect(clicEnFoto(foto(3))).toBe(true);
    expect(seleccionadas()).toEqual([100, 103]);
    expect(clicEnFoto(foto(0))).toBe(true);
    expect(seleccionadas()).toEqual([103]);
  });

  it("al desmarcar la última, el clic vuelve a abrir el visor", () => {
    clicEnFoto(foto(0), { ctrl: true });
    clicEnFoto(foto(0));
    expect(clicEnFoto(foto(1))).toBe(false);
  });

  it("Mayús + clic selecciona el rango desde la última marcada", async () => {
    clicEnFoto(foto(1), { ctrl: true });
    expect(clicEnFoto(foto(7), { mayus: true })).toBe(true);
    await esperar();
    expect(seleccionadas()).toEqual([101, 102, 103, 104, 105, 106, 107]);
    // El rango lo da el núcleo con los dos extremos, no las páginas cargadas.
    expect(biblioteca.seleccionarRango).toHaveBeenCalledWith(
      { vista: "fotos" },
      { date: 999, id: 101 },
      { date: 993, id: 107 },
    );
  });

  it("Mayús + clic hacia atrás y encadenado desde el último extremo", async () => {
    clicEnFoto(foto(5), { ctrl: true });
    await seleccionarHasta(foto(3));
    expect(seleccionadas()).toEqual([103, 104, 105]);
    await seleccionarHasta(foto(1));
    expect(seleccionadas()).toEqual([101, 102, 103, 104, 105]);
  });

  it("Mayús + clic sin foto marcada antes es como Ctrl + clic", async () => {
    clicEnFoto(foto(4), { mayus: true });
    await esperar();
    expect(seleccionadas()).toEqual([104]);
    expect(biblioteca.seleccionarRango).not.toHaveBeenCalled();
  });

  it("Ctrl + A selecciona toda la vista", async () => {
    expect(teclaSeleccion("a", { ctrl: true })).toBe(true);
    await esperar();
    expect(seleccion.ids.size).toBe(10);
    expect(biblioteca.seleccionarTodo).toHaveBeenCalledWith({ vista: "fotos" });
  });

  it("Ctrl + A en una búsqueda selecciona sus resultados", async () => {
    cambiarAmbito({ busqueda: "playa" });
    await seleccionarVista();
    expect(biblioteca.seleccionarTodo).toHaveBeenCalledWith({
      busqueda: "playa",
    });
  });

  it("A sin Ctrl no hace nada", () => {
    expect(teclaSeleccion("a")).toBe(false);
    expect(biblioteca.seleccionarTodo).not.toHaveBeenCalled();
  });

  it("la casilla del día añade todo el día y lo quita si ya estaba", async () => {
    clicEnFoto(foto(0), { ctrl: true });
    await alternarDia("2024-10-12");
    expect(seleccionadas()).toEqual([100, 101, 102, 103, 104, 105]);
    expect(todasSeleccionadas([100, 101, 102, 103, 104, 105])).toBe(true);
    await alternarDia("2024-10-11");
    expect(seleccion.ids.size).toBe(10);
    await alternarDia("2024-10-12");
    expect(seleccionadas()).toEqual([106, 107, 108, 109]);
  });

  it("un día a medias se completa en vez de vaciarse", async () => {
    clicEnFoto(foto(7), { ctrl: true });
    await alternarDia("2024-10-11");
    expect(seleccionadas()).toEqual([106, 107, 108, 109]);
  });

  it("Esc quita la selección y, sin ella, hace lo de siempre", () => {
    clicEnFoto(foto(0), { ctrl: true });
    expect(teclaSeleccion("Escape")).toBe(true);
    expect(haySeleccion()).toBe(false);
    expect(seleccion.ancla).toBeNull();
    expect(teclaSeleccion("Escape")).toBe(false);
  });

  it("cambiar de vista o buscar pierde la selección", () => {
    clicEnFoto(foto(0), { ctrl: true });
    cambiarAmbito({ vista: "fotos" });
    expect(haySeleccion()).toBe(true);
    cambiarAmbito({ vista: "favoritos" });
    expect(haySeleccion()).toBe(false);
    clicEnFoto(foto(0), { ctrl: true });
    cambiarAmbito({ busqueda: "playa" });
    expect(haySeleccion()).toBe(false);
  });

  it("una respuesta que llega tras cambiar de vista se descarta", async () => {
    clicEnFoto(foto(0), { ctrl: true });
    const pendiente = seleccionarVista();
    cambiarAmbito({ vista: "archivo" });
    await pendiente;
    expect(haySeleccion()).toBe(false);
  });

  it("si el núcleo falla, la selección se conserva y se avisa", async () => {
    const avisos: unknown[] = [];
    alFallar((e) => avisos.push(e));
    vi.mocked(biblioteca.seleccionarRango).mockRejectedValueOnce(
      new Error("disco desconectado"),
    );
    clicEnFoto(foto(1), { ctrl: true });
    clicEnFoto(foto(5), { mayus: true });
    await esperar();
    expect(seleccionadas()).toEqual([101]);
    expect(avisos).toHaveLength(1);
  });
});
