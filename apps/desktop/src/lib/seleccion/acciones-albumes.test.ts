import { beforeEach, describe, expect, it, vi } from "vitest";

// Núcleo simulado: un álbum (7) con las fotos que tenga `enAlbum`.
const enAlbum = new Set<number>();

vi.mock("../biblioteca", () => ({
  anadirAAlbum: vi.fn(async (_album: number, ids: number[]) => {
    const nuevas = ids.filter((id) => !enAlbum.has(id));
    nuevas.forEach((id) => enAlbum.add(id));
    return nuevas;
  }),
  quitarDeAlbum: vi.fn(async (_album: number, ids: number[]) => {
    const estaban = ids.filter((id) => enAlbum.has(id));
    estaban.forEach((id) => enAlbum.delete(id));
    return estaban;
  }),
  crearAlbum: vi.fn(async (nombre: string, ids: number[]) => {
    if (!nombre.trim()) throw new Error("el nombre del álbum no puede estar vacío");
    return { id: 9, name: nombre.trim(), photos: ids.length };
  }),
  seleccionarTodo: vi.fn(),
  seleccionarDia: vi.fn(),
  seleccionarRango: vi.fn(),
}));

import { crearAlbum } from "../biblioteca";
import { navegacion } from "../navegacion.svelte";
import {
  albumAbierto,
  anadirSeleccionAAlbum,
  avisos,
  cerrarAviso,
  crearAlbumConSeleccion,
  deshacer,
  quitarSeleccionDeAlbum,
} from "./acciones.svelte";
import { alternar, cambiarAmbito, seleccion } from "./seleccion.svelte";

const marcar = (...ids: number[]) => ids.forEach((id) => alternar({ id, date: 0 }));

describe("álbumes desde la barra de selección", () => {
  beforeEach(() => {
    enAlbum.clear();
    cerrarAviso();
    cambiarAmbito({ vista: "fotos" });
  });

  it("solo hay álbum abierto en la vista de un álbum", () => {
    expect(albumAbierto()).toBeNull();
    cambiarAmbito({ vista: { album: 7 } });
    expect(albumAbierto()).toBe(7);
    cambiarAmbito({ busqueda: "playa" });
    expect(albumAbierto()).toBeNull();
  });

  it("añadir cuenta solo las nuevas y ofrece «Ver álbum»", async () => {
    enAlbum.add(2);
    marcar(1, 2, 3);
    await anadirSeleccionAAlbum(7, "Picos de Europa");
    expect([...enAlbum].sort()).toEqual([1, 2, 3]);
    expect(seleccion.ids.size).toBe(0);
    expect(avisos.actual?.texto).toBe("2 fotos añadidas a «Picos de Europa»");
    expect(avisos.actual?.etiqueta).toBe("Ver álbum");
    await deshacer();
    expect(navegacion.seccion).toBe("albumes");
    expect(navegacion.album).toBe(7);
  });

  it("si ya estaban todas, lo dice", async () => {
    enAlbum.add(1);
    marcar(1);
    await anadirSeleccionAAlbum(7, "Gatos");
    expect(avisos.actual?.texto).toBe("Ya estaban en «Gatos»");
  });

  it("«Nuevo álbum…» crea el álbum con la selección", async () => {
    marcar(4, 5);
    expect(await crearAlbumConSeleccion("Ruta del Cares")).toBeNull();
    expect(crearAlbum).toHaveBeenCalledWith("Ruta del Cares", [4, 5]);
    expect(avisos.actual?.texto).toBe("2 fotos añadidas a «Ruta del Cares»");
  });

  it("si crear falla, devuelve el error y conserva la selección", async () => {
    marcar(4);
    expect(await crearAlbumConSeleccion(" ")).toMatch(/vacío/);
    expect(seleccion.ids.size).toBe(1);
  });

  it("quitar del álbum se puede deshacer", async () => {
    cambiarAmbito({ vista: { album: 7 } });
    [1, 2, 3].forEach((id) => enAlbum.add(id));
    marcar(1, 3);
    await quitarSeleccionDeAlbum();
    expect([...enAlbum]).toEqual([2]);
    expect(avisos.actual?.texto).toBe("2 fotos quitadas del álbum");
    await deshacer();
    expect([...enAlbum].sort()).toEqual([1, 2, 3]);
  });
});
