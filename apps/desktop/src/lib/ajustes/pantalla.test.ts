import { describe, expect, it } from "vitest";
import { esTeclaPantallaCompleta } from "./pantalla";

const tecla = (
  key: string,
  mods: Partial<
    Record<"ctrlKey" | "altKey" | "metaKey" | "shiftKey", boolean>
  > = {},
) => ({
  key,
  ctrlKey: false,
  altKey: false,
  metaKey: false,
  shiftKey: false,
  ...mods,
});

describe("esTeclaPantallaCompleta", () => {
  it("F11 sola alterna la pantalla completa", () => {
    expect(esTeclaPantallaCompleta(tecla("F11"))).toBe(true);
  });

  it("F11 con modificadores no", () => {
    expect(esTeclaPantallaCompleta(tecla("F11", { ctrlKey: true }))).toBe(
      false,
    );
    expect(esTeclaPantallaCompleta(tecla("F11", { shiftKey: true }))).toBe(
      false,
    );
    expect(esTeclaPantallaCompleta(tecla("F11", { altKey: true }))).toBe(false);
  });

  it("Ctrl + Cmd + F, el atajo de macOS, también", () => {
    expect(
      esTeclaPantallaCompleta(tecla("f", { ctrlKey: true, metaKey: true })),
    ).toBe(true);
    expect(
      esTeclaPantallaCompleta(tecla("F", { ctrlKey: true, metaKey: true })),
    ).toBe(true);
  });

  it("la F suelta o con un solo modificador no (favorita, buscar)", () => {
    expect(esTeclaPantallaCompleta(tecla("f"))).toBe(false);
    expect(esTeclaPantallaCompleta(tecla("f", { ctrlKey: true }))).toBe(false);
    expect(esTeclaPantallaCompleta(tecla("f", { metaKey: true }))).toBe(false);
    expect(esTeclaPantallaCompleta(tecla("F10"))).toBe(false);
  });
});
