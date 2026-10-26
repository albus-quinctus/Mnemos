import { describe, expect, it } from "vitest";
import { CacheImagenes, vecinas } from "./cache";

function cache(maximo = 5) {
  const cargadas: number[] = [];
  const soltadas: number[] = [];
  const c = new CacheImagenes<number>(
    maximo,
    (id) => (cargadas.push(id), id),
    (v) => soltadas.push(v),
  );
  return { c, cargadas, soltadas };
}

describe("CacheImagenes", () => {
  it("no carga dos veces la misma imagen", () => {
    const { c, cargadas } = cache();
    c.obtener(1);
    c.obtener(1);
    expect(cargadas).toEqual([1]);
  });

  it("guarda como mucho 5 imágenes y suelta las más antiguas", () => {
    const { c, soltadas } = cache();
    for (let id = 1; id <= 7; id++) c.obtener(id);
    expect(c.ids).toEqual([3, 4, 5, 6, 7]);
    expect(soltadas).toEqual([1, 2]);
  });

  it("usar una imagen la vuelve reciente", () => {
    const { c } = cache(3);
    c.obtener(1);
    c.obtener(2);
    c.obtener(3);
    c.obtener(1);
    c.obtener(4);
    expect(c.ids).toEqual([3, 1, 4]);
  });

  it("al precargar nunca suelta la actual ni sus vecinas", () => {
    const { c } = cache(3);
    for (let id = 1; id <= 3; id++) c.obtener(id);
    c.precargar([10, 11, 9]);
    expect(c.ids.sort((a, b) => a - b)).toEqual([9, 10, 11]);
  });

  it("vaciar suelta todo", () => {
    const { c, soltadas } = cache();
    c.obtener(1);
    c.obtener(2);
    c.vaciar();
    expect(c.ids).toEqual([]);
    expect(soltadas.sort()).toEqual([1, 2]);
  });
});

describe("vecinas", () => {
  it("la actual, la siguiente y la anterior, sin salirse", () => {
    expect(vecinas(5, 10)).toEqual([5, 6, 4]);
    expect(vecinas(0, 10)).toEqual([0, 1]);
    expect(vecinas(9, 10)).toEqual([9, 8]);
  });
});
