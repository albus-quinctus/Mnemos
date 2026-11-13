import { describe, expect, it } from "vitest";
import { cuandoSeEliminan, diasRestantes, enviadasEl, seEliminaraEl } from "./fechas";

const hoy = new Date(2026, 10, 12, 18, 30); // 12-11-2026, por la tarde

describe("fechas de la papelera", () => {
  it("cuenta los días que quedan desde el día del envío", () => {
    expect(diasRestantes("2026-11-12", hoy)).toBe(30);
    expect(diasRestantes("2026-11-02", hoy)).toBe(20);
    expect(diasRestantes("2026-10-14", hoy)).toBe(1);
    expect(diasRestantes("2026-10-01", hoy)).toBe(0);
  });

  it("dice cuándo se eliminan", () => {
    expect(cuandoSeEliminan("2026-11-02", hoy)).toBe("se eliminan en 20 días");
    expect(cuandoSeEliminan("2026-10-14", hoy)).toBe("se eliminan mañana");
    expect(cuandoSeEliminan("2026-10-13", hoy)).toBe("se eliminan hoy");
  });

  it("escribe la fecha de envío sin el año actual", () => {
    expect(enviadasEl("2026-11-12", hoy)).toBe("Enviadas el 12 nov");
    expect(enviadasEl("2025-12-30", hoy)).toBe("Enviadas el 30 dic 2025");
  });

  it("da el día en que se eliminará una foto", () => {
    const enviada = new Date(2026, 10, 12, 9).getTime() / 1000;
    expect(seEliminaraEl(enviada, hoy)).toBe("Se eliminará el 12 dic");
  });
});
