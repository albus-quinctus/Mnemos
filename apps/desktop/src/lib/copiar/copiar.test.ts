import { describe, expect, it } from "vitest";
import {
  carpetaDe,
  copiaTexto,
  esTeclaCopiar,
  esTeclaDescargar,
  esTeclaGuardarComo,
  nombreDe,
  unir,
} from "./copiar";

const tecla = (key: string, mods: Partial<KeyboardEvent> = {}) => ({
  key,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
});

describe("atajos", () => {
  it("Ctrl + C copia, con Ctrl o Cmd, y nada más", () => {
    expect(esTeclaCopiar(tecla("c", { ctrlKey: true }))).toBe(true);
    expect(esTeclaCopiar(tecla("C", { metaKey: true }))).toBe(true);
    expect(esTeclaCopiar(tecla("c"))).toBe(false);
    expect(esTeclaCopiar(tecla("C", { ctrlKey: true, shiftKey: true }))).toBe(false);
  });

  it("Mayús + D descarga, pero no con Ctrl", () => {
    expect(esTeclaDescargar(tecla("D", { shiftKey: true }))).toBe(true);
    expect(esTeclaDescargar(tecla("d"))).toBe(false);
    expect(esTeclaDescargar(tecla("D", { shiftKey: true, ctrlKey: true }))).toBe(false);
  });

  it("Ctrl + Mayús + S guarda como", () => {
    expect(esTeclaGuardarComo(tecla("S", { shiftKey: true, ctrlKey: true }))).toBe(true);
    expect(esTeclaGuardarComo(tecla("s", { ctrlKey: true }))).toBe(false);
  });
});

describe("copiar texto en vez de la foto", () => {
  it("en un campo o con texto seleccionado", () => {
    const campo = { closest: (s: string) => (s.includes("input") ? {} : null) };
    const foto = { closest: () => null };
    expect(copiaTexto(campo as unknown as EventTarget, "")).toBe(true);
    expect(copiaTexto(foto as unknown as EventTarget, "IMG_2003.jpg")).toBe(true);
    expect(copiaTexto(foto as unknown as EventTarget, "  ")).toBe(false);
    expect(copiaTexto(null, "")).toBe(false);
  });
});

describe("rutas", () => {
  it("separa nombre y carpeta en Linux y Windows", () => {
    expect(nombreDe("/home/lilith/Descargas/IMG 1.jpg")).toBe("IMG 1.jpg");
    expect(nombreDe("C:\\Users\\Paulo\\Downloads\\a.jpg")).toBe("a.jpg");
    expect(carpetaDe("/home/lilith/Descargas/a.jpg")).toBe("/home/lilith/Descargas");
    expect(carpetaDe("C:\\Users\\a.jpg")).toBe("C:\\Users");
    expect(carpetaDe("a.jpg")).toBe("");
  });

  it("une con el separador de la carpeta", () => {
    expect(unir(undefined, "a.jpg")).toBe("a.jpg");
    expect(unir("/home/lilith", "a.jpg")).toBe("/home/lilith/a.jpg");
    expect(unir("/home/lilith/", "a.jpg")).toBe("/home/lilith/a.jpg");
    expect(unir("C:\\Users\\Paulo", "a.jpg")).toBe("C:\\Users\\Paulo\\a.jpg");
  });
});
