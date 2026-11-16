// Textos de la pantalla Explorar (spec 021), sin estado, para probarlos solos.
import type { Coleccion, Vista } from "$lib/biblioteca";
import type { NombreIcono } from "$lib/componentes/Icono.svelte";

const numero = (n: number) => n.toLocaleString("es");

/** «Hace 1 año» o «Hace 3 años». */
export function haceAnios(n: number): string {
  return n === 1 ? "Hace 1 año" : `Hace ${numero(n)} años`;
}

/** «42 elementos» o «1 elemento». */
export function textoElementos(n: number): string {
  return `${numero(n)} ${n === 1 ? "elemento" : "elementos"}`;
}

const mesCorto = (d: Date) => d.toLocaleDateString("es", { month: "short" }).replace(/\./g, "");

/**
 * Días de un recuerdo, `[desde, hasta)` en segundos Unix: «12–18 nov 2025»,
 * «28 oct – 3 nov 2025» o «29 dic 2024 – 4 ene 2025».
 */
export function textoRango(desde: number, hasta: number): string {
  const a = new Date(desde * 1000);
  // `hasta` es la medianoche del día siguiente al último.
  const b = new Date((hasta - 1) * 1000);
  if (a.getFullYear() !== b.getFullYear()) {
    return `${a.getDate()} ${mesCorto(a)} ${a.getFullYear()} – ${b.getDate()} ${mesCorto(b)} ${b.getFullYear()}`;
  }
  if (a.getMonth() !== b.getMonth()) {
    return `${a.getDate()} ${mesCorto(a)} – ${b.getDate()} ${mesCorto(b)} ${b.getFullYear()}`;
  }
  return `${a.getDate()}–${b.getDate()} ${mesCorto(b)} ${b.getFullYear()}`;
}

function corta(fecha: Date, hoy: Date): string {
  return fecha
    .toLocaleDateString("es", {
      day: "numeric",
      month: "short",
      year: fecha.getFullYear() === hoy.getFullYear() ? undefined : "numeric",
    })
    .replace(/\./g, "");
}

/** «Última importación: 14 nov» (con el año si no es el actual). */
export function ultimaImportacion(segundos: number, hoy = new Date()): string {
  return `Última importación: ${corta(new Date(segundos * 1000), hoy)}`;
}

/** Cabecera de un día en Añadidas recientemente: «Importadas el 14 nov». */
export function importadasEl(dia: string, hoy = new Date()): string {
  const [anio, mes, d] = dia.split("-").map(Number);
  return `Importadas el ${corta(new Date(anio, mes - 1, d), hoy)}`;
}

export const COLECCIONES: Record<Coleccion, { nombre: string; icono: NombreIcono }> = {
  videos: { nombre: "Vídeos", icono: "video" },
  capturas: { nombre: "Capturas de pantalla", icono: "captura" },
  panoramicas: { nombre: "Panorámicas", icono: "panoramica" },
  editadas: { nombre: "Editadas", icono: "editar" },
  raw: { nombre: "RAW", icono: "raw" },
  recientes: { nombre: "Añadidas recientemente", icono: "reciente" },
};

/** La colección de una vista, o `null` si no es una colección. */
export function coleccionDe(vista: Vista): Coleccion | null {
  return typeof vista === "object" && "coleccion" in vista ? vista.coleccion : null;
}

/** Vistas que se abren desde Explorar: una colección o un recuerdo. */
export function esVistaExplorar(vista: Vista): boolean {
  return typeof vista === "object" && ("coleccion" in vista || "rango" in vista);
}
