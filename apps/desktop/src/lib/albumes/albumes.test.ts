import { describe, expect, it } from "vitest";
import { nombreValido, rangoFechas, subtituloAlbum, textoAlbumes, textoFotos } from "./albumes";

const dia = (a: number, m: number, d: number) => new Date(a, m - 1, d, 12).getTime() / 1000;
const corto = (a: number, m: number, d: number) =>
  new Date(a, m - 1, d).toLocaleDateString("es", { day: "numeric", month: "short" });

describe("álbumes", () => {
  it("cuenta álbumes en singular y plural", () => {
    expect(textoAlbumes(0)).toBe("Ningún álbum");
    expect(textoAlbumes(1)).toBe("1 álbum");
    expect(textoAlbumes(13)).toBe("13 álbumes");
    expect(textoAlbumes(1200)).toBe("1200 álbumes".replace("1200", (1200).toLocaleString("es")));
  });

  it("cuenta fotos y dice si está vacío", () => {
    expect(textoFotos(0)).toBe("Vacío");
    expect(textoFotos(1)).toBe("1 foto");
    expect(textoFotos(48)).toBe("48 fotos");
  });

  it("no deja crear un álbum sin nombre", () => {
    expect(nombreValido("   ")).toBeNull();
    expect(nombreValido("  Picos de Europa ")).toBe("Picos de Europa");
  });

  it("escribe el rango de fechas del álbum", () => {
    expect(rangoFechas(null, null)).toBe("");
    expect(rangoFechas(dia(2024, 9, 7), dia(2024, 9, 7))).toBe(`${corto(2024, 9, 7)} 2024`);
    expect(rangoFechas(dia(2024, 8, 18), dia(2024, 9, 7))).toBe(
      `${corto(2024, 8, 18)} – ${corto(2024, 9, 7)} 2024`,
    );
    expect(rangoFechas(dia(2023, 12, 3), dia(2024, 1, 5))).toBe(
      `${corto(2023, 12, 3)} 2023 – ${corto(2024, 1, 5)} 2024`,
    );
  });

  it("junta número y fechas en el subtítulo", () => {
    expect(subtituloAlbum(0, null, null)).toBe("Vacío");
    expect(subtituloAlbum(2, dia(2024, 9, 7), dia(2024, 9, 7))).toBe(`2 fotos · ${corto(2024, 9, 7)} 2024`);
  });
});
