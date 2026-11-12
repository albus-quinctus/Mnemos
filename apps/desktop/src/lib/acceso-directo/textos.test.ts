import { describe, expect, it } from "vitest";
import { pista } from "./textos";

describe("textos del acceso directo", () => {
  it("dice dónde aparecerá según el sistema", () => {
    expect(pista("linux", "no")).toBe("MNEMOS aparecerá en el menú de aplicaciones.");
    expect(pista("windows", "no")).toBe("MNEMOS aparecerá en Inicio y en el escritorio.");
  });

  it("dice dónde está cuando ya existe", () => {
    expect(pista("linux", "si")).toBe("Ya está en el menú de aplicaciones.");
    expect(pista("windows", "si")).toBe("Ya está en Inicio y en el escritorio.");
  });

  it("avisa de que no toca un archivo ajeno", () => {
    expect(pista("linux", "ajeno")).toContain("no es de MNEMOS; no se toca");
    expect(pista("windows", "ajeno")).toContain("no es de MNEMOS; no se toca");
  });
});
