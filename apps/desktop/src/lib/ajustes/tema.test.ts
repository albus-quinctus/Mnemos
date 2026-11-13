// @ts-expect-error sin el paquete @types/node (como en vite.config.js)
import { readFileSync } from "node:fs";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Núcleo simulado: guarda el tema o falla (disco en solo lectura).
let guardado: string | null = null;
let fallar = false;

vi.mock("../biblioteca", async (original) => ({
  ...(await original<typeof import("../biblioteca")>()),
  ponerTema: vi.fn(async (tema: string) => {
    if (fallar) throw new Error("no se pudo guardar la configuración");
    guardado = tema;
  }),
}));

import { avisos, cerrarAviso } from "../seleccion/acciones.svelte";
import {
  AVISO_NO_GUARDADO,
  aplicarTema,
  elegirTema,
  temaActual,
} from "./tema.svelte";

// Sin DOM en las pruebas: basta con un `dataset`.
const html = () => ({ dataset: {} as DOMStringMap }) as HTMLElement;

beforeEach(() => {
  guardado = null;
  fallar = false;
  cerrarAviso();
});

describe("tema", () => {
  it("Claro y Oscuro ponen data-tema; Sistema lo quita", () => {
    const raiz = html();
    aplicarTema("oscuro", raiz);
    expect(raiz.dataset.tema).toBe("oscuro");
    expect(temaActual(raiz)).toBe("oscuro");
    aplicarTema("claro", raiz);
    expect(temaActual(raiz)).toBe("claro");
    aplicarTema("sistema", raiz);
    expect(raiz.dataset.tema).toBeUndefined();
    expect(temaActual(raiz)).toBe("sistema");
  });

  it("un valor desconocido en data-tema se lee como Sistema", () => {
    const raiz = html();
    raiz.dataset.tema = "sepia";
    expect(temaActual(raiz)).toBe("sistema");
  });

  it("elegir un tema lo aplica al momento y lo guarda", async () => {
    const raiz = html();
    await elegirTema("oscuro", raiz);
    expect(temaActual(raiz)).toBe("oscuro");
    expect(guardado).toBe("oscuro");
    expect(avisos.actual).toBeNull();
  });

  it("si no se puede guardar, sigue aplicado y avisa", async () => {
    fallar = true;
    const raiz = html();
    await elegirTema("claro", raiz);
    expect(temaActual(raiz)).toBe("claro");
    expect(avisos.actual?.texto).toBe(AVISO_NO_GUARDADO);
    expect(avisos.actual?.error).toBe(true);
  });
});

describe("tokens.css", () => {
  // Vitest no sirve el CSS (`?raw` llega vacío): se lee del disco.
  const css: string = readFileSync(
    new URL("../estilos/tokens.css", import.meta.url),
    "utf8",
  );
  const variables = (bloque: string) =>
    [...bloque.matchAll(/(--[\w-]+):\s*([^;]+);/g)].map(
      ([, n, v]) => `${n}: ${v}`,
    );

  it("el oscuro del sistema y el oscuro elegido tienen los mismos colores", () => {
    const sistema = css.match(
      /:root:not\(\[data-tema="claro"\]\)\s*\{([^}]*)\}/,
    )?.[1];
    const elegido = css.match(/:root\[data-tema="oscuro"\]\s*\{([^}]*)\}/)?.[1];
    expect(sistema).toBeDefined();
    expect(elegido).toBeDefined();
    expect(variables(elegido!)).toEqual(variables(sistema!));
    expect(variables(sistema!).length).toBeGreaterThan(5);
  });
});
