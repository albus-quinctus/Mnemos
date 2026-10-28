import { describe, expect, it } from "vitest";
import { dimensiones, fecha, fechaCaptura, tamano } from "./formato";

describe("formato", () => {
  it("tamaño del archivo", () => {
    expect(tamano(512)).toBe("512 bytes");
    expect(tamano(4_200_000)).toBe("4,2 MB");
    expect(tamano(25_300)).toBe("25,3 KB");
    expect(tamano(150_000_000)).toBe("150 MB");
  });

  it("dimensiones con megapíxeles", () => {
    expect(dimensiones(4000, 3000)).toBe("4000 × 3000 · 12 MP");
    expect(dimensiones(1920, 1080)).toBe("1920 × 1080 · 2,1 MP");
    expect(dimensiones(null, 3000)).toBe("Desconocidas");
  });

  it("fecha y hora locales", () => {
    const segundos = new Date(2026, 9, 26, 14, 3).getTime() / 1000;
    expect(fecha(segundos)).toMatch(/^26 oct 2026,? 14:03$/);
  });

  it("fecha de captura con su origen", () => {
    const segundos = new Date(2019, 4, 12, 14, 30).getTime() / 1000;
    expect(fechaCaptura(segundos, "exif").texto).toMatch(/^12 may 2019,? 14:30$/);
    expect(fechaCaptura(segundos, "exif").origen).toBe("Según la cámara (EXIF)");
    expect(fechaCaptura(segundos, "nombre_dia").texto).toBe("12 may 2019");
    expect(fechaCaptura(null, "ninguna")).toEqual({ texto: "Sin fecha de captura", origen: null });
    expect(fechaCaptura(segundos, null).origen).toBeNull();
  });
});
