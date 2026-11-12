// Acceso directo en el equipo (spec 012). Solo se ofrece en modo portátil, en
// Linux y Windows: `disponible` lo decide el núcleo, y fuera de Tauri (o si
// falla) la entrada no se muestra.

import {
  crearAccesoDirecto,
  estadoAccesoDirecto,
  quitarAccesoDirecto,
  type EstadoAccesoDirecto,
  type InformeAccesoDirecto,
  type SistemaAccesoDirecto,
} from "../biblioteca";
import { mostrarAviso } from "../seleccion/acciones.svelte";
import { AVISO_ANADIDO, AVISO_QUITADO } from "./textos";

export const acceso = $state({
  disponible: false,
  estado: "no" as EstadoAccesoDirecto,
  sistema: "otro" as SistemaAccesoDirecto,
  ocupado: false,
});

function aplicar(informe: InformeAccesoDirecto) {
  acceso.disponible = informe.disponible;
  acceso.estado = informe.estado;
  acceso.sistema = informe.sistema;
}

/** Pregunta al núcleo si se ofrece el acceso directo y si ya existe. */
export async function cargarAcceso() {
  try {
    aplicar(await estadoAccesoDirecto());
  } catch {
    acceso.disponible = false;
  }
}

function mensaje(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** «Añadir a este equipo»: crea el acceso directo y avisa, con «Quitar». */
export async function anadir() {
  if (acceso.ocupado) return;
  acceso.ocupado = true;
  try {
    aplicar(await crearAccesoDirecto());
    mostrarAviso({
      texto: AVISO_ANADIDO,
      etiqueta: "Quitar",
      deshacer: async () => aplicar(await quitarAccesoDirecto()),
    });
  } catch (e) {
    mostrarAviso({ texto: `No se ha podido añadir: ${mensaje(e)}`, error: true });
    await cargarAcceso();
  } finally {
    acceso.ocupado = false;
  }
}

/** «Quitar»: borra el acceso directo de este equipo. */
export async function quitar() {
  if (acceso.ocupado) return;
  acceso.ocupado = true;
  try {
    aplicar(await quitarAccesoDirecto());
    mostrarAviso({ texto: AVISO_QUITADO });
  } catch (e) {
    mostrarAviso({ texto: `No se ha podido quitar: ${mensaje(e)}`, error: true });
    await cargarAcceso();
  } finally {
    acceso.ocupado = false;
  }
}
