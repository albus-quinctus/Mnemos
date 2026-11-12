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
  favorite: boolean; // en Favoritos: estrella (spec 006)
  archived: boolean; // archivada: caja en la búsqueda (spec 006)
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

export function fotos(
  despues: Cursor | null,
  limite: number,
): Promise<PaginaFotos> {
  return invoke("fotos", { despues, limite });
}

// Vistas, selección, favoritos, archivo y títulos de día (spec 006). Reflejan
// `mnemos_core::{View, Scope, DayTitle}`.
export type Vista = "fotos" | "favoritos" | "archivo";

// Dónde se selecciona: una vista o los resultados de una búsqueda.
export type Ambito = { vista: Vista } | { busqueda: string };

export interface TituloDia {
  day: string; // "2024-10-12", en hora local
  title: string;
}

export function vistaMeses(vista: Vista): Promise<Mes[]> {
  return invoke("vista_meses", { vista });
}

export function vistaFotos(
  vista: Vista,
  despues: Cursor | null,
  limite: number,
): Promise<PaginaFotos> {
  return invoke("vista_fotos", { vista, despues, limite });
}

// Devuelven los ids que han cambiado de verdad: aplicar lo contrario solo a
// esos es «Deshacer».
export function marcarFavoritos(
  ids: number[],
  favorito: boolean,
): Promise<number[]> {
  return invoke("marcar_favoritos", { ids, favorito });
}

export function archivar(ids: number[], archivar: boolean): Promise<number[]> {
  return invoke("archivar", { ids, archivar });
}

// Ids en el orden de la cuadrícula, sin cargar páginas.
export function seleccionarTodo(ambito: Ambito): Promise<number[]> {
  return invoke("seleccionar_todo", { ambito });
}

export function seleccionarDia(ambito: Ambito, dia: string): Promise<number[]> {
  return invoke("seleccionar_dia", { ambito, dia });
}

export function seleccionarRango(
  ambito: Ambito,
  desde: Cursor,
  hasta: Cursor,
): Promise<number[]> {
  return invoke("seleccionar_rango", { ambito, desde, hasta });
}

export function titulosDia(): Promise<TituloDia[]> {
  return invoke("titulos_dia");
}

// Devuelve el título como ha quedado (limpio y cortado a 100 caracteres), o
// null si se ha borrado.
export function ponerTituloDia(
  dia: string,
  titulo: string,
): Promise<string | null> {
  return invoke("poner_titulo_dia", { dia, titulo });
}

// Búsqueda (spec 003): los mismos tipos que la cuadrícula, solo con los
// resultados de `texto`. El núcleo interpreta el texto; un texto vacío da la
// biblioteca entera.
export function buscarMeses(texto: string): Promise<Mes[]> {
  return invoke("buscar_meses", { texto });
}

export function buscarFotos(
  texto: string,
  despues: Cursor | null,
  limite: number,
): Promise<PaginaFotos> {
  return invoke("buscar_fotos", { texto, despues, limite });
}

// Lo que emite la cola de miniaturas (`mnemos_core::ThumbnailEvent`).
export type EventoMiniaturas =
  | { tipo: "ready"; photo_ids: number[] }
  | { tipo: "failed"; photo_ids: number[]; message: string }
  | { tipo: "stopped"; message: string };

export function priorizarMiniaturas(ids: number[]): Promise<void> {
  return invoke("priorizar_miniaturas", { ids });
}

// Detalles de una foto para el panel del visor (`mnemos_core::PhotoDetails`).
export interface Detalles {
  rel_path: string;
  width: number | null; // ya girada según la orientación
  height: number | null;
  size_bytes: number;
  modified_at: number; // segundos desde la época Unix
  taken_at: number | null; // fecha de captura (spec 004)
  taken_source: string | null; // "exif", "nombre", "takeout", "nombre_dia", "manual", "ninguna" o null
  taken_auto_at: number | null; // con fecha a mano, la automática (spec 011)
  taken_auto_source: string | null;
  format: string; // "JPEG", "PNG", "DNG"…
}

export function detalles(id: number): Promise<Detalles | null> {
  return invoke("detalles", { id });
}

// Datos de cámara del EXIF (`mnemos_imaging::CameraInfo`, spec 011).
export interface Camara {
  camera: string | null;
  lens: string | null;
  exposure_s: number | null;
  f_number: number | null;
  iso: number | null;
  focal_mm: number | null;
  focal_35mm: number | null;
  flash: boolean | null;
  gps: [number, number] | null; // latitud y longitud en grados
}

export function camara(id: number): Promise<Camara | null> {
  return invoke("camara", { id });
}

// Fecha de captura a mano: `fecha` es la hora local "AAAA-MM-DD hh:mm[:ss]".
// Devuelven false si la foto ya no existe.
export function ponerFecha(id: number, fecha: string): Promise<boolean> {
  return invoke("poner_fecha", { id, fecha });
}

export function quitarFechaManual(id: number): Promise<boolean> {
  return invoke("quitar_fecha_manual", { id });
}

// Exportar una foto (spec 008). "original" copia el archivo tal cual;
// "reducida" guarda un JPEG de 2048 px como mucho, sin metadatos. Devuelve la
// ruta del archivo creado; nunca sobrescribe nada en `carpeta`.
export type TamanoExportacion = "original" | "reducida";

export function exportar(
  id: number,
  carpeta: string,
  tamano: TamanoExportacion,
): Promise<string> {
  return invoke("exportar", { id, carpeta, tamano });
}

// Editor no destructivo (spec 009). La geometría es la de la receta
// (`mnemos_imaging::recipe::Geometry`): giro en grados en sentido horario
// sobre la foto ya girada según el EXIF, volteo después del giro y recorte en
// fracciones (0 a 1) de la imagen ya girada y volteada.
export interface Recorte {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Geometria {
  rotate: 0 | 90 | 180 | 270;
  flip_horizontal: boolean;
  crop: Recorte | null;
}

// Una versión guardada (`mnemos_core::EditVersion`).
export interface VersionEdicion {
  id: number;
  created_at: number; // segundos desde la época Unix
  rotate: number;
  flip_horizontal: boolean;
  cropped: boolean;
  crop: Recorte | null;
}

// Ediciones de una foto (`mnemos_core::EditSummary`).
export interface Ediciones {
  current: number | null; // versión que se ve, o null si se ve el original
  versions: VersionEdicion[]; // de la más antigua a la más nueva
  damaged: string | null; // por qué no se puede usar la receta
  writable: boolean; // false si la receta es de una versión más nueva
}

export function ediciones(id: number): Promise<Ediciones> {
  return invoke("ediciones", { id });
}

// «Guardar como original»: añade una versión y la hace actual. Devuelve su
// número. El archivo original no se toca.
export function guardarEdicion(
  id: number,
  geometria: Geometria,
): Promise<number> {
  return invoke("guardar_edicion", { id, geometria });
}

// «Guardar como copia»: crea `nombre (N).ext` junto al original con la
// edición aplicada y devuelve el id de la foto nueva.
export function guardarCopia(
  id: number,
  geometria: Geometria,
): Promise<number> {
  return invoke("guardar_copia", { id, geometria });
}

// Muestra una versión, o el original con `null` («Volver al original»).
export function elegirVersion(
  id: number,
  version: number | null,
): Promise<void> {
  return invoke("elegir_version", { id, version });
}

// Duplicados exactos (spec 010). Reflejan `mnemos_core::duplicates`. Solo
// consultan: nada se borra ni se mueve.
export interface ResumenDuplicados {
  groups: number;
  extra_copies: number; // en cada grupo, todas las copias menos una
  extra_bytes: number;
}

export interface CopiaDuplicada {
  id: number;
  rel_path: string;
  size_bytes: number;
  date: number; // captura o, si falta, modificación
  favorite: boolean;
  archived: boolean;
  exists: boolean; // false: foto desaparecida del disco
}

export interface GrupoDuplicado {
  content_hash: string;
  date: number;
  copies: CopiaDuplicada[]; // por orden de llegada a la biblioteca
}

export interface CursorDuplicados {
  date: number;
  content_hash: string;
}

export interface PaginaDuplicados {
  groups: GrupoDuplicado[];
  next: CursorDuplicados | null;
}

export function duplicadosResumen(): Promise<ResumenDuplicados> {
  return invoke("duplicados_resumen");
}

export function duplicadosGrupos(
  despues: CursorDuplicados | null,
  limite: number,
): Promise<PaginaDuplicados> {
  return invoke("duplicados_grupos", { despues, limite });
}
