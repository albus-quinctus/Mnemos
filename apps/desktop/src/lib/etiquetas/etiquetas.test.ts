import { describe, expect, it } from "vitest";
import {
  casilla,
  existeNombre,
  filtrarEtiquetas,
  nombreValido,
  ordenarEtiquetas,
  resumenCambios,
  textoEtiquetas,
  textoFotos,
} from "./etiquetas";

const e = (...nombres: string[]) => nombres.map((name) => ({ name }));
const nombres = (l: { name: string }[]) => l.map((x) => x.name);

describe("etiquetas", () => {
  it("cuenta en singular y plural", () => {
    expect(textoEtiquetas(0)).toBe("Ninguna etiqueta");
    expect(textoEtiquetas(1)).toBe("1 etiqueta");
    expect(textoEtiquetas(11)).toBe("11 etiquetas");
    expect(textoFotos(0)).toBe("0 fotos");
    expect(textoFotos(1)).toBe("1 foto");
    expect(textoFotos(318)).toBe("318 fotos");
  });

  it("limpia el nombre como el núcleo", () => {
    expect(nombreValido("  Ruta   del\tCares ")).toBe("Ruta del Cares");
    expect(nombreValido("   ")).toBeNull();
    expect(nombreValido("ñ".repeat(50))).toBe("ñ".repeat(50));
    expect(nombreValido("ñ".repeat(51))).toBeNull();
    expect(nombreValido("🏖️ playa")).toBe("🏖️ playa");
  });

  it("ordena con las reglas del español", () => {
    expect(nombres(ordenarEtiquetas(e("Playa", "ávila", "Abuela", "Navidad", "Ñu", "Nube")))).toEqual([
      "Abuela",
      "ávila",
      "Navidad",
      "Nube",
      "Ñu",
      "Playa",
    ]);
    expect(nombres(ordenarEtiquetas(e("Viaje 10", "Viaje 2")))).toEqual(["Viaje 2", "Viaje 10"]);
  });

  it("filtra sin tildes ni mayúsculas, con las que empiezan delante", () => {
    const lista = e("Montaña", "Playa", "Asturias", "Cumpleaños", "Málaga");
    expect(nombres(filtrarEtiquetas(lista, "MA"))).toEqual(["Málaga"]);
    expect(nombres(filtrarEtiquetas(lista, "an"))).toEqual(["Montaña", "Cumpleaños"]);
    expect(nombres(filtrarEtiquetas(lista, "PL"))).toEqual(["Playa", "Cumpleaños"]);
    expect(nombres(filtrarEtiquetas(lista, "  "))).toEqual(nombres(lista));
    expect(filtrarEtiquetas(lista, "xyz")).toEqual([]);
  });

  it("sabe si un nombre ya existe sin mirar mayúsculas", () => {
    const lista = e("Playa", "Ávila");
    expect(existeNombre(lista, " playa ")).toBe(true);
    expect(existeNombre(lista, "ÁVILA")).toBe(true);
    expect(existeNombre(lista, "Avila")).toBe(false);
  });
});

describe("etiquetar una selección", () => {
  it("elige la casilla según cuántas la tienen", () => {
    expect(casilla(0, 7)).toBe("no");
    expect(casilla(3, 7)).toBe("algo");
    expect(casilla(7, 7)).toBe("si");
    expect(casilla(1, 1)).toBe("si");
  });

  it("resume lo que se ha cambiado", () => {
    expect(resumenCambios([])).toBeNull();
    expect(resumenCambios([{ nombre: "Playa", puesta: true, fotos: [] }])).toBeNull();
    expect(resumenCambios([{ nombre: "Playa", puesta: true, fotos: [1, 2] }])).toBe(
      "Etiqueta «Playa» añadida a 2 fotos",
    );
    expect(resumenCambios([{ nombre: "Playa", puesta: false, fotos: [1] }])).toBe(
      "Etiqueta «Playa» quitada de 1 foto",
    );
    expect(
      resumenCambios([
        { nombre: "Playa", puesta: true, fotos: [1, 2] },
        { nombre: "Abuela", puesta: true, fotos: [2, 3] },
      ]),
    ).toBe("Etiquetas cambiadas en 3 fotos");
  });
});
