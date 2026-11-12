// Acciones sobre la selección (spec 006): favoritos y archivo, con un aviso
// de 8 s para deshacer. Cada acción va al núcleo en una sola transacción: si
// falla, nada cambia y la selección se conserva para reintentar.

import { archivar, marcarFavoritos, type FotoCuadricula } from "../biblioteca";
import { avisarCambio } from "../cambios.svelte";
import { alFallar, limpiarSeleccion, seleccion } from "./seleccion.svelte";

export const DURACION_AVISO = 8000;

export interface Aviso {
  texto: string;
  deshacer?: () => Promise<void>;
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

// «1 foto archivada», «3 fotos archivadas».
function fotosTexto(n: number, participio: string): string {
  return n === 1 ? `1 foto ${participio}` : `${n.toLocaleString("es")} fotos ${participio}s`;
}

async function ejecutar(accion: () => Promise<void>) {
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

// Tras una acción correcta se quita la selección y las cuadrículas recargan.
function terminar(texto: string, revertir: () => Promise<number[]>) {
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
