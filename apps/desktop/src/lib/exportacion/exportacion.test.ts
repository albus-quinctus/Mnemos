import { beforeEach, describe, expect, it, vi } from "vitest";

const elegida = { valor: "/home/ana/Pendrive" as string | null };
const fallo = { mensaje: null as string | null };

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => elegida.valor),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("../biblioteca", () => ({
  exportarVarias: vi.fn(async () => {
    if (fallo.mensaje) throw fallo.mensaje;
  }),
  cancelarExportacion: vi.fn(async () => {}),
}));

import { open } from "@tauri-apps/plugin-dialog";
import * as biblioteca from "../biblioteca";
import {
  cerrarExportacion,
  destino,
  exportacion,
  exportarVarias,
} from "./exportacion.svelte";

describe("exportar varias", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    exportacion.actual = null;
    destino.ultima = undefined;
    elegida.valor = "/home/ana/Pendrive";
    fallo.mensaje = null;
  });

  it("pide la carpeta y empieza con los ids y el tamaño", async () => {
    await exportarVarias([3, 1, 2], "reducida");
    expect(open).toHaveBeenCalledWith(
      expect.objectContaining({ directory: true, title: "Elige dónde guardar las 3 fotos" }),
    );
    expect(biblioteca.exportarVarias).toHaveBeenCalledWith([3, 1, 2], "/home/ana/Pendrive", "reducida");
    expect(exportacion.actual).toMatchObject({ estado: "en_curso", total: 3, hechas: 0 });
    expect(destino.ultima).toBe("/home/ana/Pendrive");
  });

  it("con una sola foto el selector habla de la foto", async () => {
    await exportarVarias([7], "original");
    expect(open).toHaveBeenCalledWith(expect.objectContaining({ title: "Elige dónde guardar la foto" }));
  });

  it("si se cancela el selector no pasa nada", async () => {
    elegida.valor = null;
    await exportarVarias([1, 2], "original");
    expect(biblioteca.exportarVarias).not.toHaveBeenCalled();
    expect(exportacion.actual).toBeNull();
  });

  it("propone la última carpeta usada", async () => {
    destino.ultima = "/media/fotos";
    await exportarVarias([1, 2], "original");
    expect(open).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "/media/fotos" }));
  });

  it("no empieza otra mientras hay una en marcha", async () => {
    await exportarVarias([1, 2], "original");
    await exportarVarias([4, 5], "original");
    expect(biblioteca.exportarVarias).toHaveBeenCalledTimes(1);
  });

  it("si no puede empezar queda detenida con el motivo", async () => {
    fallo.mensaje = "no se puede exportar en /x: está dentro de la biblioteca";
    await exportarVarias([1, 2], "original");
    expect(exportacion.actual).toMatchObject({
      estado: "detenida",
      mensaje: "No se puede exportar en /x: está dentro de la biblioteca",
    });
  });

  it("la tarjeta no se cierra mientras exporta", async () => {
    await exportarVarias([1, 2], "original");
    cerrarExportacion();
    expect(exportacion.actual).not.toBeNull();
    exportacion.actual = { ...exportacion.actual!, estado: "terminada", hechas: 2, guardadas: 2 };
    cerrarExportacion();
    expect(exportacion.actual).toBeNull();
  });
});
