import { describe, expect, it } from "vitest";
import type { GrupoDuplicado } from "$lib/biblioteca";
import { diaLocal, fechaCopia, fotosDelGrupo, separarRuta, textoResumen } from "./duplicados";

const mediodia = (a: number, m: number, d: number) => new Date(a, m - 1, d, 12).getTime() / 1000;

describe("duplicados", () => {
  it("separa carpeta y nombre", () => {
    expect(separarRuta("Originales/b ñ/IMG 1.jpg")).toEqual({
      carpeta: "Originales/b ñ/",
      nombre: "IMG 1.jpg",
    });
    expect(separarRuta("raiz.png")).toEqual({ carpeta: "", nombre: "raiz.png" });
  });

  it("escribe el resumen en singular y plural", () => {
    expect(textoResumen({ groups: 1, extra_copies: 1, extra_bytes: 4_200_000 })).toBe(
      "1 grupo · 1 copia de más · 4,2 MB",
    );
    expect(textoResumen({ groups: 3, extra_copies: 4, extra_bytes: 13_300_000 })).toBe(
      "3 grupos · 4 copias de más · 13,3 MB",
    );
  });

  it("da el día en hora local", () => {
    expect(diaLocal(mediodia(2024, 8, 17))).toBe("2024-08-17");
    const copia = {
      id: 1,
      rel_path: "a.jpg",
      size_bytes: 1,
      date: mediodia(2024, 8, 17),
      favorite: false,
      archived: false,
      exists: true,
    };
    expect(fechaCopia(copia, new Date(2026, 10, 5))).toBe("Sáb, 17 ago 2024");
  });

  it("convierte las copias en fotos para el visor", () => {
    const grupo: GrupoDuplicado = {
      content_hash: "ab",
      date: mediodia(2024, 8, 17),
      copies: [
        {
          id: 7,
          rel_path: "Originales/x/IMG.jpg",
          size_bytes: 10,
          date: mediodia(2024, 8, 17),
          favorite: true,
          archived: false,
          exists: false,
        },
      ],
    };
    expect(fotosDelGrupo(grupo)).toEqual([
      {
        id: 7,
        name: "IMG.jpg",
        month: "2024-08",
        date: grupo.date,
        width: null,
        height: null,
        thumbnail_failed: false,
        favorite: true,
        archived: false,
      },
    ]);
  });
});
