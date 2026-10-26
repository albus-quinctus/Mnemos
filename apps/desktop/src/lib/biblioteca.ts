import { invoke } from "@tauri-apps/api/core";

// Tipos y comandos de la cuadrícula (spec 002). Reflejan `mnemos_core::timeline`.

export interface Mes {
  month: string; // "2026-10", en hora local
  photos: number;
}

export interface FotoCuadricula {
  id: number;
  name: string; // nombre del archivo, sin carpetas
  month: string;
  date: number; // segundos desde la época Unix
  width: number | null; // ya girada según la orientación
  height: number | null;
  thumbnail_failed: boolean; // sin miniatura posible: se muestra el marcador
}

export interface Cursor {
  date: number;
  id: number;
}

export interface PaginaFotos {
  photos: FotoCuadricula[];
  next: Cursor | null; // null cuando no quedan más
}

export function meses(): Promise<Mes[]> {
  return invoke("meses");
}

export function fotos(despues: Cursor | null, limite: number): Promise<PaginaFotos> {
  return invoke("fotos", { despues, limite });
}

// Lo que emite la cola de miniaturas (`mnemos_core::ThumbnailEvent`).
export type EventoMiniaturas =
  | { tipo: "ready"; photo_ids: number[] }
  | { tipo: "failed"; photo_ids: number[]; message: string }
  | { tipo: "stopped"; message: string };

export function priorizarMiniaturas(ids: number[]): Promise<void> {
  return invoke("priorizar_miniaturas", { ids });
}
