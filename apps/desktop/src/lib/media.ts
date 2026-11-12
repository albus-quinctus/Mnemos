import { convertFileSrc } from "@tauri-apps/api/core";

// URLs del protocolo `mnemos://` (spec 002). Siempre por id, nunca por ruta.
// `convertFileSrc` elige la forma de cada sistema: `mnemos://localhost/...`
// o, en Windows, `http://mnemos.localhost/...`.

// `version` cambia la URL tras guardar una edición (spec 009), para que el
// motor web no reutilice la imagen que ya tiene en memoria.
const conVersion = (url: string, version?: number) => (version ? `${url}?v=${version}` : url);

export function miniaturaUrl(id: number, version?: number): string {
  return conVersion(convertFileSrc(`miniatura/${id}`, "mnemos"), version);
}

export function fotoUrl(id: number, version?: number): string {
  return conVersion(convertFileSrc(`foto/${id}`, "mnemos"), version);
}

/** La foto sin la edición, sobre la que trabaja el editor. */
export function originalUrl(id: number): string {
  return convertFileSrc(`original/${id}`, "mnemos");
}
