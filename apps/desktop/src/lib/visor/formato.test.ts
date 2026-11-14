import { describe, expect, it } from "vitest";
import {
  duracion,
  apertura,
  cambiosVersion,
  camposFecha,
  coordenadas,
  cuandoVersion,
  dimensiones,
  exposicion,
  fecha,
  fechaCaptura,
  fechaManual,
  focal,
  tamano,
} from "./formato";

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

  it("fecha puesta a mano", () => {
    const segundos = new Date(2019, 4, 12, 14, 30).getTime() / 1000;
    expect(fechaCaptura(segundos, "manual").origen).toBe("Puesta a mano");
  });

  it("datos de la cámara", () => {
    expect(exposicion(0.004)).toBe("1/250 s");
    expect(exposicion(0.5)).toBe("1/2 s");
    expect(exposicion(2)).toBe("2 s");
    expect(exposicion(1.5)).toBe("1,5 s");
    expect(apertura(2.8)).toBe("ƒ/2,8");
    expect(apertura(8)).toBe("ƒ/8");
    expect(focal(4.25, 26)).toBe("4,3 mm (26 mm en 35 mm)");
    expect(focal(50, 50)).toBe("50 mm");
    expect(focal(50, null)).toBe("50 mm");
    expect(focal(null, 28)).toBe("28 mm en 35 mm");
    expect(focal(null, null)).toBeNull();
    expect(coordenadas([41.3851, -2.1734])).toBe("41,38510° N, 2,17340° O");
    expect(coordenadas([-33.8688, 151.2093])).toBe("33,86880° S, 151,20930° E");
  });

  it("campos del formulario de fecha", () => {
    const segundos = new Date(2019, 4, 2, 9, 5, 7).getTime() / 1000;
    expect(camposFecha(segundos)).toEqual({ dia: "2019-05-02", hora: "09:05:07" });
    expect(fechaManual("2019-05-02", "09:05")).toBe("2019-05-02 09:05:00");
    expect(fechaManual("2019-05-02", "09:05:07")).toBe("2019-05-02 09:05:07");
    expect(fechaManual("", "09:05")).toBeNull();
    expect(fechaManual("2019-05-02", "")).toBeNull();
  });
});

describe("versiones de una edición", () => {
  it("dice qué cambia", () => {
    const v = { rotate: 0, flip_horizontal: false, cropped: false };
    expect(cambiosVersion(v)).toBe("sin cambios");
    expect(cambiosVersion({ ...v, cropped: true })).toBe("recorte");
    expect(cambiosVersion({ ...v, cropped: true, rotate: 90 })).toBe("recorte y giro");
    expect(cambiosVersion({ rotate: 270, flip_horizontal: true, cropped: true })).toBe(
      "recorte, giro y volteo",
    );
  });

  it("dice cuándo se guardó", () => {
    const ahora = new Date(2026, 10, 12, 20, 0);
    const s = (d: Date) => d.getTime() / 1000;
    expect(cuandoVersion(s(new Date(2026, 10, 12, 18, 20)), ahora)).toBe("hoy, 18:20");
    expect(cuandoVersion(s(new Date(2026, 10, 11, 21, 5)), ahora)).toBe("ayer, 21:05");
    expect(cuandoVersion(s(new Date(2026, 9, 3, 9, 0)), ahora)).toContain("2026");
  });
});

describe("duracion", () => {
  it("minutos y segundos, y horas si las hay", () => {
    expect(duracion(42_000)).toBe("0:42");
    expect(duracion(125_000)).toBe("2:05");
    expect(duracion(3_910_000)).toBe("1:05:10");
    expect(duracion(400)).toBe("0:00");
  });
});
