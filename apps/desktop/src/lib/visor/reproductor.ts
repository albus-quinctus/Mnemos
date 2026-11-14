// Reproductor de vídeo del visor (spec 019): volumen y silencio que se
// recuerdan durante la sesión, y teclas propias.

export const sonido = { volumen: 1, silencio: false };

/** Segundos que salta `Mayús` + flecha. */
export const SALTO_S = 5;

/** Qué hace una tecla con el vídeo, o `null` si no es suya. */
export function accionVideo(
  e: Pick<KeyboardEvent, "key" | "shiftKey">,
): "alternar" | "silenciar" | "atras" | "adelante" | null {
  if (e.key === " ") return "alternar";
  if ((e.key === "m" || e.key === "M") && !e.shiftKey) return "silenciar";
  if (e.shiftKey && e.key === "ArrowLeft") return "atras";
  if (e.shiftKey && e.key === "ArrowRight") return "adelante";
  return null;
}

/** Aplica la acción al `<video>`. */
export function aplicarAccion(video: HTMLVideoElement, accion: ReturnType<typeof accionVideo>) {
  switch (accion) {
    case "alternar":
      if (video.paused) void video.play().catch(() => {});
      else video.pause();
      break;
    case "silenciar":
      video.muted = !video.muted;
      break;
    case "atras":
      video.currentTime = Math.max(0, video.currentTime - SALTO_S);
      break;
    case "adelante":
      video.currentTime = Math.min(video.duration || Infinity, video.currentTime + SALTO_S);
      break;
  }
}
