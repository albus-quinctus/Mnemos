// Papelera (spec 015): confirmar antes de borrar, seguir el borrado y el
// vaciado automático. Enviar y restaurar están en `seleccion/acciones`.

import { listen } from "@tauri-apps/api/event";
import {
  eliminarParaSiempre,
  vaciadoAutomatico,
  vaciarPapelera,
  type EventoPapelera,
} from "../biblioteca";
import { avisarCambio } from "../cambios.svelte";
import { mostrarAviso } from "../seleccion/acciones.svelte";
import { limpiarSeleccion } from "../seleccion/seleccion.svelte";

export interface Confirmacion {
  titulo: string;
  texto: string;
  boton: string;
  aceptar: () => Promise<void>;
}

// El diálogo que se ve, si hay uno. Lo pinta `Confirmar.svelte`.
export const confirmacion = $state({ actual: null as Confirmacion | null });

// Si hay un borrado en marcha: los botones de borrar se desactivan.
export const borrado = $state({ enCurso: false });

const fotos = (n: number) => (n === 1 ? "1 foto" : `${n.toLocaleString("es")} fotos`);

/** «Eliminar para siempre» sobre la selección o desde el visor. */
export function pedirEliminar(ids: number[]) {
  if (ids.length === 0 || borrado.enCurso) return;
  confirmacion.actual = {
    titulo: `¿Eliminar ${fotos(ids.length)} para siempre?`,
    texto: "Se borrarán del disco y no se podrán recuperar.",
    boton: "Eliminar para siempre",
    aceptar: () => empezar(() => eliminarParaSiempre(ids)),
  };
}

/** «Vaciar papelera». `total` es lo que hay en la vista. */
export function pedirVaciar(total: number) {
  if (total === 0 || borrado.enCurso) return;
  confirmacion.actual = {
    titulo: "¿Vaciar la papelera?",
    texto: `Se borrarán del disco ${fotos(total)} y no se podrán recuperar.`,
    boton: "Vaciar papelera",
    aceptar: () => empezar(vaciarPapelera),
  };
}

export function cancelarConfirmacion() {
  confirmacion.actual = null;
}

export async function aceptarConfirmacion() {
  const actual = confirmacion.actual;
  confirmacion.actual = null;
  await actual?.aceptar();
}

async function empezar(orden: () => Promise<void>) {
  borrado.enCurso = true;
  try {
    await orden();
  } catch (e) {
    borrado.enCurso = false;
    mostrarAviso({ texto: `No se ha podido eliminar: ${String(e)}`, error: true });
  }
}

/** Qué hacer con cada evento `papelera` del núcleo. */
export function alEvento(e: EventoPapelera) {
  switch (e.estado) {
    case "en_curso":
      borrado.enCurso = true;
      if (e.total > 1)
        mostrarAviso({
          texto: `Eliminando ${e.hechas.toLocaleString("es")} de ${e.total.toLocaleString("es")}…`,
        });
      return;
    case "terminado": {
      borrado.enCurso = false;
      if (e.eliminadas.length > 0) {
        limpiarSeleccion();
        avisarCambio();
      }
      const fallidas = e.fallidas.length;
      if (e.motivo === "automatico" && e.eliminadas.length === 0 && fallidas === 0) return;
      let texto =
        e.motivo === "automatico"
          ? `Se eliminaron ${fotos(e.eliminadas.length)} que llevaban 30 días en la papelera`
          : `${fotos(e.eliminadas.length)} ${e.eliminadas.length === 1 ? "eliminada" : "eliminadas"}`;
      if (fallidas > 0)
        texto += `. ${fotos(fallidas)} no se ${fallidas === 1 ? "pudo" : "pudieron"} borrar y ${
          fallidas === 1 ? "sigue" : "siguen"
        } en la papelera: ${e.fallidas[0].motivo}`;
      mostrarAviso({ texto, error: fallidas > 0 });
      return;
    }
    case "reloj_adelantado":
      borrado.enCurso = false;
      confirmacion.actual = {
        titulo: `¿Eliminar ${fotos(e.caducadas)} de la papelera?`,
        texto:
          "La fecha de este equipo va muy por delante de la última vez que se abrió la " +
          "biblioteca. Si es correcta, estas fotos llevan 30 días en la papelera y se " +
          "borrarán del disco. Si no lo es, cancela y corrige la fecha.",
        boton: "Eliminar para siempre",
        aceptar: () => empezar(() => vaciadoAutomatico(true)),
      };
      return;
    case "fallido":
      borrado.enCurso = false;
      mostrarAviso({ texto: `No se ha podido eliminar: ${e.mensaje ?? ""}`, error: true });
      return;
  }
}

let escuchando = false;

/** Recibe los avisos del núcleo. Se llama una vez al arrancar. */
export function escucharPapelera(): void {
  if (escuchando) return;
  escuchando = true;
  listen<EventoPapelera>("papelera", (e) => alEvento(e.payload)).catch(
    () => (escuchando = false),
  );
}

const UNA_HORA = 3_600_000;
let temporizador: ReturnType<typeof setInterval> | undefined;

/** Vaciado automático al abrir una biblioteca y después cada hora. */
export function vaciarCaducadas(): void {
  clearInterval(temporizador);
  const pedir = () => vaciadoAutomatico().catch(() => {});
  pedir();
  temporizador = setInterval(pedir, UNA_HORA);
}
