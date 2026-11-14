import { describe, expect, it } from "vitest";
import { accionVideo, aplicarAccion } from "./reproductor";

describe("accionVideo", () => {
  it("reconoce las teclas del vídeo", () => {
    expect(accionVideo({ key: " ", shiftKey: false })).toBe("alternar");
    expect(accionVideo({ key: "m", shiftKey: false })).toBe("silenciar");
    expect(accionVideo({ key: "ArrowLeft", shiftKey: true })).toBe("atras");
    expect(accionVideo({ key: "ArrowRight", shiftKey: true })).toBe("adelante");
  });
  it("las flechas solas siguen pasando de elemento", () => {
    expect(accionVideo({ key: "ArrowRight", shiftKey: false })).toBeNull();
    expect(accionVideo({ key: "M", shiftKey: true })).toBeNull();
  });
});

describe("aplicarAccion", () => {
  function video(currentTime: number, duration: number) {
    return { currentTime, duration, muted: false, paused: true } as HTMLVideoElement;
  }
  it("salta 5 s sin salirse del vídeo", () => {
    const v = video(3, 10);
    aplicarAccion(v, "atras");
    expect(v.currentTime).toBe(0);
    aplicarAccion(v, "adelante");
    aplicarAccion(v, "adelante");
    aplicarAccion(v, "adelante");
    expect(v.currentTime).toBe(10);
  });
  it("silencia y vuelve a dar sonido", () => {
    const v = video(0, 10);
    aplicarAccion(v, "silenciar");
    expect(v.muted).toBe(true);
    aplicarAccion(v, "silenciar");
    expect(v.muted).toBe(false);
  });
});
