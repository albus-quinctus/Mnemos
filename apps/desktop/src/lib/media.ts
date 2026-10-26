import { convertFileSrc } from "@tauri-apps/api/core";

// URLs del protocolo `mnemos://` (spec 002). Siempre por id, nunca por ruta.
// `convertFileSrc` elige la forma de cada sistema: `mnemos://localhost/...`
// o, en Windows, `http://mnemos.localhost/...`.

export function miniaturaUrl(id: number): string {
  return convertFileSrc(`miniatura/${id}`, "mnemos");
}

export function fotoUrl(id: number): string {
  return convertFileSrc(`foto/${id}`, "mnemos");
}
