import { describe, expect, it } from "vitest";
import { textoCopiadas } from "./importacion.svelte";

describe("textoCopiadas", () => {
  it("sin vídeos cuenta copiadas, como antes", () => {
    expect(textoCopiadas(0, 0)).toBe("0 copiadas");
    expect(textoCopiadas(12, 0)).toBe("12 copiadas");
  });

  it("con vídeos cuenta fotos y vídeos aparte", () => {
    expect(textoCopiadas(134, 14)).toBe("120 fotos y 14 vídeos copiados");
    expect(textoCopiadas(2, 1)).toBe("1 foto y 1 vídeo copiados");
  });

  it("solo vídeos", () => {
    expect(textoCopiadas(1, 1)).toBe("1 vídeo copiado");
    expect(textoCopiadas(3, 3)).toBe("3 vídeos copiados");
  });
});
