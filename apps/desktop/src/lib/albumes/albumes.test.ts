import { describe, expect, it } from "vitest";
import { nombreValido, textoAlbumes, textoFotos } from "./albumes";

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
});
