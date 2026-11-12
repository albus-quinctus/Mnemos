import { beforeEach, describe, expect, it, vi } from "vitest";

// Núcleo simulado: el equipo tiene o no el acceso directo.
let existe = false;
let fallar = false;
let disponible = true;
const informe = () => ({
  disponible,
  estado: existe ? ("si" as const) : ("no" as const),
  sistema: "linux" as const,
});

vi.mock("../biblioteca", async (original) => ({
  ...(await original<typeof import("../biblioteca")>()),
  estadoAccesoDirecto: vi.fn(async () => {
    if (fallar) throw new Error("sin Tauri");
    return informe();
  }),
  crearAccesoDirecto: vi.fn(async () => {
    if (fallar) throw new Error("Ya hay un mnemos-portatil.desktop que no es de MNEMOS");
    existe = true;
    return informe();
  }),
  quitarAccesoDirecto: vi.fn(async () => {
    existe = false;
    return informe();
  }),
}));

import { avisos, cerrarAviso, deshacer } from "../seleccion/acciones.svelte";
import { acceso, anadir, cargarAcceso, quitar } from "./acceso.svelte";
import { AVISO_ANADIDO, AVISO_QUITADO } from "./textos";

describe("acceso directo en el equipo", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    existe = false;
    fallar = false;
    disponible = true;
    acceso.disponible = false;
    acceso.estado = "no";
    acceso.ocupado = false;
    cerrarAviso();
  });

  it("lee del núcleo si se ofrece y si ya existe", async () => {
    existe = true;
    await cargarAcceso();
    expect(acceso).toMatchObject({ disponible: true, estado: "si", sistema: "linux" });
  });

  it("si el núcleo falla (fuera de Tauri), no se ofrece", async () => {
    acceso.disponible = true;
    fallar = true;
    await cargarAcceso();
    expect(acceso.disponible).toBe(false);
  });

  it("añadir lo crea y avisa, con «Quitar»", async () => {
    await cargarAcceso();
    await anadir();
    expect(acceso.estado).toBe("si");
    expect(avisos.actual).toMatchObject({ texto: AVISO_ANADIDO, etiqueta: "Quitar" });
    expect(avisos.actual?.error).toBeUndefined();
  });

  it("el «Quitar» del aviso lo deshace", async () => {
    await cargarAcceso();
    await anadir();
    await deshacer();
    expect(existe).toBe(false);
    expect(acceso.estado).toBe("no");
    expect(avisos.actual).toBeNull();
  });

  it("si no se puede crear, lo dice y vuelve a mirar el estado", async () => {
    await cargarAcceso();
    fallar = true;
    await anadir();
    expect(avisos.actual?.error).toBe(true);
    expect(avisos.actual?.texto).toContain("No se ha podido añadir: Ya hay un");
    expect(acceso.ocupado).toBe(false);
  });

  it("quitar lo borra y avisa", async () => {
    existe = true;
    await cargarAcceso();
    await quitar();
    expect(acceso.estado).toBe("no");
    expect(avisos.actual).toMatchObject({ texto: AVISO_QUITADO });
  });

  it("no lanza dos veces la misma acción a la vez", async () => {
    const { crearAccesoDirecto } = await import("../biblioteca");
    await cargarAcceso();
    await Promise.all([anadir(), anadir()]);
    expect(crearAccesoDirecto).toHaveBeenCalledTimes(1);
  });
});
