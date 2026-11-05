import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Núcleo simulado con el estado de 6 fotos: devuelve los ids que cambian.
const estado = new Map<number, { favorite: boolean; archived: boolean }>();
let fallar = false;

vi.mock("../biblioteca", () => ({
  marcarFavoritos: vi.fn(async (ids: number[], favorito: boolean) => {
    if (fallar) throw new Error("disco desconectado");
    const cambian = ids.filter((id) => estado.get(id)!.favorite !== favorito);
    for (const id of cambian) estado.get(id)!.favorite = favorito;
    return cambian;
  }),
  archivar: vi.fn(async (ids: number[], archivar: boolean) => {
    if (fallar) throw new Error("disco desconectado");
    const cambian = ids.filter((id) => estado.get(id)!.archived !== archivar);
    for (const id of cambian) estado.get(id)!.archived = archivar;
    return cambian;
  }),
  seleccionarTodo: vi.fn(),
  seleccionarDia: vi.fn(),
  seleccionarRango: vi.fn(),
}));

import type { FotoCuadricula } from "../biblioteca";
import { cambios } from "../cambios.svelte";
import {
  alternarArchivo,
  alternarFavoritos,
  avisos,
  cerrarAviso,
  deshacer,
  registrarFotos,
  todasFavoritas,
} from "./acciones.svelte";
import { alternar, cambiarAmbito, limpiarSeleccion, seleccion } from "./seleccion.svelte";

const marcar = (...ids: number[]) => ids.forEach((id) => alternar({ id, date: 0 }));
const favoritas = () => [...estado].filter(([, e]) => e.favorite).map(([id]) => id);
const archivadas = () => [...estado].filter(([, e]) => e.archived).map(([id]) => id);

// La cuadrícula solo tiene cargadas las fotos 1 a 4.
const cargadas = (id: number) =>
  id <= 4 ? ({ id, favorite: estado.get(id)!.favorite } as FotoCuadricula) : undefined;
const quitarFuente = registrarFotos(cargadas);

describe("acciones de la selección", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    fallar = false;
    for (let id = 1; id <= 6; id++) estado.set(id, { favorite: false, archived: false });
    cambiarAmbito({ vista: "fotos" });
    limpiarSeleccion();
    cerrarAviso();
  });
  afterEach(() => vi.useRealTimers());

  it("la estrella marca todas aunque alguna ya fuera favorita", async () => {
    estado.get(1)!.favorite = true;
    marcar(1, 2, 3);
    expect(todasFavoritas()).toBe(false);
    await alternarFavoritos();
    expect(favoritas()).toEqual([1, 2, 3]);
    expect(avisos.actual?.texto).toBe("2 fotos añadidas a Favoritos");
    expect(seleccion.ids.size).toBe(0);
  });

  it("si todas eran favoritas, la estrella las quita", async () => {
    estado.get(1)!.favorite = estado.get(2)!.favorite = true;
    marcar(1, 2);
    expect(todasFavoritas()).toBe(true);
    await alternarFavoritos();
    expect(favoritas()).toEqual([]);
    expect(avisos.actual?.texto).toBe("2 fotos quitadas de Favoritos");
  });

  it("con fotos sin cargar que ya eran favoritas, también las quita", async () => {
    estado.get(5)!.favorite = estado.get(6)!.favorite = true;
    marcar(5, 6);
    await alternarFavoritos();
    expect(favoritas()).toEqual([]);
  });

  it("deshacer devuelve solo las que cambiaron a su estado anterior", async () => {
    estado.get(1)!.favorite = true;
    marcar(1, 2, 3);
    await alternarFavoritos();
    await deshacer();
    expect(favoritas()).toEqual([1]);
    expect(avisos.actual).toBeNull();
  });

  it("archivar y deshacer; en Archivo, desarchivar", async () => {
    marcar(2, 4);
    await alternarArchivo();
    expect(archivadas()).toEqual([2, 4]);
    expect(avisos.actual?.texto).toBe("2 fotos archivadas");
    await deshacer();
    expect(archivadas()).toEqual([]);

    estado.get(3)!.archived = true;
    cambiarAmbito({ vista: "archivo" });
    marcar(3);
    await alternarArchivo();
    expect(archivadas()).toEqual([]);
    expect(avisos.actual?.texto).toBe("1 foto desarchivada");
  });

  it("las cuadrículas recargan tras cada acción y tras deshacer", async () => {
    const antes = cambios.version;
    marcar(1);
    await alternarArchivo();
    await deshacer();
    expect(cambios.version).toBe(antes + 2);
  });

  it("el aviso desaparece a los 8 s", async () => {
    marcar(1);
    await alternarArchivo();
    vi.advanceTimersByTime(7999);
    expect(avisos.actual).not.toBeNull();
    vi.advanceTimersByTime(1);
    expect(avisos.actual).toBeNull();
  });

  it("si el núcleo falla, la selección se conserva y sale un error", async () => {
    marcar(1, 2);
    fallar = true;
    await alternarArchivo();
    expect([...seleccion.ids]).toEqual([1, 2]);
    expect(archivadas()).toEqual([]);
    expect(avisos.actual?.error).toBe(true);
    expect(avisos.actual?.texto).toContain("disco desconectado");
  });
});

void quitarFuente;
