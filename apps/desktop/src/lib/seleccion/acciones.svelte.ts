// Acciones sobre la selección (spec 006): favoritos y archivo, con un aviso
// de 8 s para deshacer. Cada acción va al núcleo en una sola transacción: si
// falla, nada cambia y la selección se conserva para reintentar.

import {
  anadirAAlbum,
  archivar,
  crearAlbum,
  enviarAPapelera,
  marcarFavoritos,
  quitarDeAlbum,
  restaurar,
  type FotoCuadricula,
} from "../biblioteca";
import { avisarCambio } from "../cambios.svelte";
import { abrirAlbum } from "../navegacion.svelte";
import { alFallar, limpiarSeleccion, seleccion } from "./seleccion.svelte";

export const DURACION_AVISO = 8000;

export interface Aviso {
  texto: string;
  /** Acción del botón del aviso. Por defecto se llama «Deshacer». */
  deshacer?: () => Promise<void>;
  etiqueta?: string;
  error?: boolean;
}

export const avisos = $state({ actual: null as Aviso | null, ocupado: false });

let temporizador: ReturnType<typeof setTimeout> | undefined;

export function mostrarAviso(aviso: Aviso) {
  clearTimeout(temporizador);
  avisos.actual = aviso;
  temporizador = setTimeout(cerrarAviso, DURACION_AVISO);
}

export function cerrarAviso() {
  clearTimeout(temporizador);
  avisos.actual = null;
}

// Los fallos de seleccionar (rango, día, Ctrl + A) también salen en el aviso.
alFallar((e) => mostrarAviso({ texto: mensajeDeError(e), error: true }));

function mensajeDeError(e: unknown): string {
  return `No se ha podido completar: ${String(e)}`;
}

// Las cuadrículas montadas dicen lo que saben de sus fotos cargadas, para que
// la barra sepa si la estrella añade o quita.
const fuentes = new Set<(id: number) => FotoCuadricula | undefined>();

export function registrarFotos(fuente: (id: number) => FotoCuadricula | undefined) {
  fuentes.add(fuente);
  return () => fuentes.delete(fuente);
}

function conocida(id: number): FotoCuadricula | undefined {
  for (const fuente of fuentes) {
    const foto = fuente(id);
    if (foto) return foto;
  }
  return undefined;
}

/** Si todas las fotos seleccionadas son favoritas según lo que hay cargado. */
export function todasFavoritas(): boolean {
  if (seleccion.ids.size === 0) return false;
  for (const id of seleccion.ids) if (!conocida(id)?.favorite) return false;
  return true;
}

/** En Archivo la acción es desarchivar; en el resto, archivar. */
export function enArchivo(): boolean {
  return "vista" in seleccion.ambito && seleccion.ambito.vista === "archivo";
}

/** En la Papelera (spec 015) la barra restaura y elimina para siempre. */
export function enPapelera(): boolean {
  return "vista" in seleccion.ambito && seleccion.ambito.vista === "papelera";
}

/** El álbum abierto (spec 014), donde la barra ofrece «Quitar del álbum». */
export function albumAbierto(): number | null {
  const ambito = seleccion.ambito;
  if (!("vista" in ambito)) return null;
  const vista = ambito.vista;
  return typeof vista === "object" && "album" in vista ? vista.album : null;
}

// «1 foto archivada», «3 fotos archivadas».
export function fotosTexto(n: number, participio: string): string {
  return n === 1 ? `1 foto ${participio}` : `${n.toLocaleString("es")} fotos ${participio}s`;
}

export async function ejecutar(accion: () => Promise<void>) {
  if (avisos.ocupado) return;
  avisos.ocupado = true;
  try {
    await accion();
  } catch (e) {
    mostrarAviso({ texto: mensajeDeError(e), error: true });
  } finally {
    avisos.ocupado = false;
  }
}

/**
 * Estrella de la barra: marca todas como favoritas; solo quita si todas lo
 * eran ya. Las fotos sin cargar no se conocen: si al marcar no cambia
 * ninguna, es que todas lo eran y se quitan.
 */
export function alternarFavoritos() {
  return ejecutar(async () => {
    const ids = [...seleccion.ids];
    let favorito = !todasFavoritas();
    let cambiadas = await marcarFavoritos(ids, favorito);
    if (favorito && cambiadas.length === 0) {
      favorito = false;
      cambiadas = await marcarFavoritos(ids, false);
    }
    terminar(
      favorito
        ? `${fotosTexto(cambiadas.length, "añadida")} a Favoritos`
        : `${fotosTexto(cambiadas.length, "quitada")} de Favoritos`,
      () => marcarFavoritos(cambiadas, !favorito),
    );
  });
}

/** Archivar o, dentro de Archivo, desarchivar. */
export function alternarArchivo() {
  return ejecutar(async () => {
    const archivada = !enArchivo();
    const cambiadas = await archivar([...seleccion.ids], archivada);
    terminar(
      fotosTexto(cambiadas.length, archivada ? "archivada" : "desarchivada"),
      () => archivar(cambiadas, !archivada),
    );
  });
}

/**
 * Envía la selección a la Papelera (spec 015). No toca los archivos, así
 * que no pide confirmación: el aviso permite deshacer.
 */
export function enviarSeleccionAPapelera() {
  return ejecutar(async () => {
    const cambiadas = await enviarAPapelera([...seleccion.ids]);
    terminar(`${fotosTexto(cambiadas.length, "movida")} a la papelera`, () =>
      restaurar(cambiadas),
    );
  });
}

/**
 * Añade la selección a un álbum (spec 014). Las que ya estaban no cuentan.
 * El aviso ofrece «Ver álbum» en lugar de «Deshacer».
 */
export function anadirSeleccionAAlbum(album: number, nombre: string) {
  return ejecutar(async () => {
    const cambiadas = await anadirAAlbum(album, [...seleccion.ids]);
    avisarAnadidas(cambiadas.length, album, nombre);
  });
}

/**
 * «Nuevo álbum…» de la barra: crea el álbum con la selección. Devuelve el
 * error para el diálogo, o null.
 */
export async function crearAlbumConSeleccion(nombre: string): Promise<string | null> {
  try {
    const album = await crearAlbum(nombre, [...seleccion.ids]);
    avisarAnadidas(album.photos, album.id, album.name);
    return null;
  } catch (e) {
    return String(e);
  }
}

function avisarAnadidas(n: number, album: number, nombre: string) {
  limpiarSeleccion();
  avisarCambio();
  mostrarAviso({
    texto:
      n === 0
        ? `Ya estaban en «${nombre}»`
        : `${fotosTexto(n, "añadida")} a «${nombre}»`,
    etiqueta: "Ver álbum",
    deshacer: async () => abrirAlbum(album),
  });
}

/** Dentro de un álbum: quita la selección del álbum, no de la biblioteca. */
export function quitarSeleccionDeAlbum() {
  const album = albumAbierto();
  if (album === null) return;
  return ejecutar(async () => {
    const cambiadas = await quitarDeAlbum(album, [...seleccion.ids]);
    terminar(`${fotosTexto(cambiadas.length, "quitada")} del álbum`, () =>
      anadirAAlbum(album, cambiadas),
    );
  });
}

/** Saca la selección de la Papelera, con «Deshacer». */
export function restaurarSeleccion() {
  return ejecutar(async () => {
    const cambiadas = await restaurar([...seleccion.ids]);
    terminar(fotosTexto(cambiadas.length, "restaurada"), () => enviarAPapelera(cambiadas));
  });
}

/** Restaurar desde el visor de la Papelera, con «Deshacer». */
export async function restaurarFoto(id: number): Promise<boolean> {
  try {
    await restaurar([id]);
  } catch (e) {
    mostrarAviso({ texto: mensajeDeError(e), error: true });
    return false;
  }
  mostrarAviso({
    texto: fotosTexto(1, "restaurada"),
    deshacer: async () => {
      await enviarAPapelera([id]);
      avisarCambio();
    },
  });
  return true;
}

// Tras una acción correcta se quita la selección y las cuadrículas recargan.
export function terminar(texto: string, revertir: () => Promise<number[]>) {
  limpiarSeleccion();
  avisarCambio();
  mostrarAviso({
    texto,
    deshacer: async () => {
      await revertir();
      avisarCambio();
    },
  });
}

/** Botón «Deshacer» del aviso. */
export function deshacer() {
  const aviso = avisos.actual;
  if (!aviso?.deshacer) return;
  cerrarAviso();
  return ejecutar(aviso.deshacer);
}

// Una sola foto, desde el visor (spec 006, tarea 11). Devuelven si se ha
// aplicado; si falla, sale el error en el aviso y nada cambia.

/** Estrella o tecla F del visor. */
export async function marcarFavorita(id: number, favorito: boolean): Promise<boolean> {
  try {
    await marcarFavoritos([id], favorito);
    return true;
  } catch (e) {
    mostrarAviso({ texto: mensajeDeError(e), error: true });
    return false;
  }
}

/** Archivar o desarchivar desde el visor, con «Deshacer» en el aviso. */
export async function archivarFoto(id: number, archivada: boolean): Promise<boolean> {
  try {
    await archivar([id], archivada);
  } catch (e) {
    mostrarAviso({ texto: mensajeDeError(e), error: true });
    return false;
  }
  mostrarAviso({
    texto: fotosTexto(1, archivada ? "archivada" : "desarchivada"),
    deshacer: async () => {
      await archivar([id], !archivada);
      avisarCambio();
    },
  });
  return true;
}

/** Enviar a la Papelera desde el visor (spec 015), con «Deshacer». */
export async function enviarFotoAPapelera(id: number): Promise<boolean> {
  try {
    await enviarAPapelera([id]);
  } catch (e) {
    mostrarAviso({ texto: mensajeDeError(e), error: true });
    return false;
  }
  mostrarAviso({
    texto: `${fotosTexto(1, "movida")} a la papelera`,
    deshacer: async () => {
      await restaurar([id]);
      avisarCambio();
    },
  });
  return true;
}

/** Tecla de enviar a la Papelera: `Supr`, o `Retroceso` (macOS no tiene `Supr`). */
export function esTeclaPapelera(e: KeyboardEvent): boolean {
  return e.key === "Delete" || e.key === "Backspace";
}
