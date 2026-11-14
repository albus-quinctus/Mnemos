import { invoke } from "@tauri-apps/api/core";
import { videoUrl } from "$lib/media";

// Miniaturas de vídeo (spec 019). El núcleo no descodifica vídeo: la interfaz
// carga cada vídeo en un `<video>` oculto, va a un fotograma, lo pinta en un
// `<canvas>` y manda el JPEG al núcleo, que lo guarda como cualquier
// miniatura. Lo que el motor web no sabe reproducir tampoco tiene miniatura:
// la cuadrícula lo enseña con la genérica.

/** Medidas de las miniaturas, como las de `mnemos_imaging`. */
export const ALTO_MINIATURA = 360;
export const ANCHO_MAX_MINIATURA = 1080;
const CALIDAD = 0.8;
/** Un vídeo que no carga en este tiempo cuenta como fallido. */
const ESPERA_MS = 15_000;
/** Vídeos que se piden al núcleo de una vez. */
const LOTE = 20;

/** Tamaño de la miniatura: 360 px de alto (o menos si el vídeo es más bajo). */
export function medidasMiniatura(ancho: number, alto: number): { ancho: number; alto: number } {
  if (!(ancho > 0 && alto > 0)) return { ancho: 0, alto: 0 };
  const escala = Math.min(1, ALTO_MINIATURA / alto, ANCHO_MAX_MINIATURA / ancho);
  return {
    ancho: Math.max(1, Math.round(ancho * escala)),
    alto: Math.max(1, Math.round(alto * escala)),
  };
}

/** Segundo del fotograma: el 1, o la mitad si dura menos de 2 s. */
export function momentoFotograma(duracion: number): number {
  if (!Number.isFinite(duracion) || duracion <= 0) return 0;
  return duracion < 2 ? duracion / 2 : 1;
}

function esperar(video: HTMLVideoElement, evento: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const limpiar = () => {
      clearTimeout(reloj);
      video.removeEventListener(evento, bien);
      video.removeEventListener("error", mal);
    };
    const bien = () => {
      limpiar();
      resolve();
    };
    const mal = () => {
      limpiar();
      reject(new Error(video.error?.message || "el motor web no puede leer este vídeo"));
    };
    const reloj = setTimeout(() => {
      limpiar();
      reject(new Error("el vídeo tarda demasiado en cargar"));
    }, ESPERA_MS);
    video.addEventListener(evento, bien);
    video.addEventListener("error", mal);
  });
}

/** Saca el fotograma del vídeo `id` como JPEG. Falla si no se puede leer. */
export async function sacarFotograma(id: number): Promise<Uint8Array> {
  const video = document.createElement("video");
  video.muted = true;
  video.playsInline = true;
  video.preload = "auto";
  // Sin esto el `<canvas>` queda bloqueado y no se puede leer.
  video.crossOrigin = "anonymous";
  try {
    const cargado = esperar(video, "loadeddata");
    video.src = videoUrl(id);
    await cargado;
    const saltado = esperar(video, "seeked");
    video.currentTime = momentoFotograma(video.duration);
    await saltado;
    // El motor web ya aplica el giro del vídeo a `videoWidth` y al dibujo.
    const { ancho, alto } = medidasMiniatura(video.videoWidth, video.videoHeight);
    if (!ancho) throw new Error("el vídeo no tiene imagen");
    const lienzo = document.createElement("canvas");
    lienzo.width = ancho;
    lienzo.height = alto;
    const ctx = lienzo.getContext("2d");
    if (!ctx) throw new Error("no se puede pintar el fotograma");
    ctx.drawImage(video, 0, 0, ancho, alto);
    const jpeg = await new Promise<Blob | null>((r) => lienzo.toBlob(r, "image/jpeg", CALIDAD));
    if (!jpeg) throw new Error("no se puede guardar el fotograma");
    return new Uint8Array(await jpeg.arrayBuffer());
  } finally {
    video.removeAttribute("src");
    video.load();
  }
}

export interface Dependencias {
  pendientes: (limite: number) => Promise<number[]>;
  sacar: (id: number) => Promise<Uint8Array>;
  guardar: (id: number, jpeg: Uint8Array) => Promise<void>;
  fallida: (id: number, motivo: string) => Promise<void>;
}

/**
 * Generador de una biblioteca: hace las miniaturas que faltan de una en una.
 * `arrancar` se puede llamar cuando se quiera (al abrir, al importar): si ya
 * está en marcha, da otra vuelta al terminar. Los vídeos que fallan no se
 * vuelven a intentar en esta sesión.
 */
export function crearGenerador(deps: Dependencias) {
  const fallidas = new Set<number>();
  let enMarcha = false;
  let otraVuelta = false;
  let parado = false;

  async function vuelta(): Promise<void> {
    for (;;) {
      const ids = (await deps.pendientes(LOTE + fallidas.size)).filter((id) => !fallidas.has(id));
      if (ids.length === 0 || parado) return;
      for (const id of ids) {
        if (parado) return;
        try {
          await deps.guardar(id, await deps.sacar(id));
        } catch (e) {
          fallidas.add(id);
          await deps.fallida(id, e instanceof Error ? e.message : String(e)).catch(() => {});
        }
      }
    }
  }

  return {
    async arrancar(): Promise<void> {
      if (parado) return;
      if (enMarcha) {
        otraVuelta = true;
        return;
      }
      enMarcha = true;
      try {
        do {
          otraVuelta = false;
          await vuelta();
        } while (otraVuelta && !parado);
      } catch {
        // Biblioteca cerrada o disco quitado: se reintenta al volver a abrir.
      } finally {
        enMarcha = false;
      }
    },
    parar(): void {
      parado = true;
    },
    fallidas,
  };
}

const deTauri: Dependencias = {
  pendientes: (limite) => invoke("miniaturas_video_pendientes", { limite }),
  sacar: sacarFotograma,
  guardar: (id, jpeg) => invoke("guardar_miniatura_video", jpeg, { headers: { id: String(id) } }),
  fallida: (id, motivo) => invoke("miniatura_video_fallida", { id, motivo }),
};

let actual: ReturnType<typeof crearGenerador> | null = null;

/** Empieza de cero con la biblioteca que se acaba de abrir. */
export function miniaturasVideoAlAbrir(): void {
  actual?.parar();
  actual = crearGenerador(deTauri);
  void actual.arrancar();
}

/** Tras importar vídeos nuevos. */
export function miniaturasVideoTrasImportar(): void {
  void actual?.arrancar();
}
