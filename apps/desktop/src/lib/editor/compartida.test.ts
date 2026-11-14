import { describe, expect, it } from "vitest";

import type { CopiaDuplicada } from "$lib/biblioteca";
import { botonAplicar, lineaCompartida, rutasAviso, tituloAviso } from "./compartida";

function copia(id: number, rel_path: string, exists = true): CopiaDuplicada {
  return { id, rel_path, size_bytes: 1, date: 0, favorite: false, archived: false, exists, media_kind: "foto", duration_ms: null };
}

describe("aviso de ediciones compartidas", () => {
  it("cambia título y botón con una o varias copias", () => {
    expect(tituloAviso(1)).toBe("Esta foto tiene otra copia idéntica");
    expect(tituloAviso(3)).toBe("Esta foto tiene otras 3 copias idénticas");
    expect(botonAplicar(1)).toBe("Aplicar a las dos");
    expect(botonAplicar(2)).toBe("Aplicar a todas");
  });

  it("enseña las rutas sin Originales/ y marca las desaparecidas", () => {
    const { rutas, resto } = rutasAviso([
      copia(1, "Originales/2024/05/playa (1).jpg"),
      copia(2, "Originales/copias/playa.jpg", false),
    ]);
    expect(rutas).toEqual([
      { texto: "2024/05/playa (1).jpg", desaparecida: false },
      { texto: "copias/playa.jpg", desaparecida: true },
    ]);
    expect(resto).toBe(0);
  });

  it("corta en tres rutas y cuenta el resto, salvo que quede una sola", () => {
    const muchas = Array.from({ length: 6 }, (_, i) => copia(i, `Originales/${i}.jpg`));
    expect(rutasAviso(muchas)).toMatchObject({ resto: 3 });
    expect(rutasAviso(muchas).rutas).toHaveLength(3);
    expect(rutasAviso(muchas.slice(0, 4))).toMatchObject({ resto: 0 });
    expect(rutasAviso(muchas.slice(0, 4)).rutas).toHaveLength(4);
  });

  it("la línea de Detalles solo sale con copias", () => {
    expect(lineaCompartida(0)).toBeNull();
    expect(lineaCompartida(1)).toBe("Compartidas con 1 copia idéntica");
    expect(lineaCompartida(4)).toBe("Compartidas con 4 copias idénticas");
  });
});
