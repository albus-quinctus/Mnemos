// Fechas de la Papelera (spec 015). En la vista Papelera la cuadrícula agrupa
// por el día en que se envió cada foto (`"2026-11-12"`, hora local).

import { DIAS_PAPELERA } from "../biblioteca";

const MS_DIA = 86_400_000;

function local(dia: string): Date {
  const [anio, mes, numero] = dia.split("-").map(Number);
  return new Date(anio, mes - 1, numero);
}

function inicio(fecha: Date): Date {
  return new Date(fecha.getFullYear(), fecha.getMonth(), fecha.getDate());
}

/** Días que quedan para que se eliminen las fotos enviadas ese día. */
export function diasRestantes(dia: string, hoy = new Date()): number {
  const pasados = Math.round((inicio(hoy).getTime() - local(dia).getTime()) / MS_DIA);
  return Math.max(0, DIAS_PAPELERA - pasados);
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

/** Cabecera de un día de la Papelera: «Enviadas el 12 nov». */
export function enviadasEl(dia: string, hoy = new Date()): string {
  return `Enviadas el ${corta(local(dia), hoy)}`;
}

/** «se eliminan en 28 días», «se eliminan mañana» o «se eliminan hoy». */
export function cuandoSeEliminan(dia: string, hoy = new Date()): string {
  const quedan = diasRestantes(dia, hoy);
  if (quedan <= 0) return "se eliminan hoy";
  if (quedan === 1) return "se eliminan mañana";
  return `se eliminan en ${quedan} días`;
}

/** Para el visor: «Se eliminará el 12 dic», a partir de la fecha de envío. */
export function seEliminaraEl(enviada: number, hoy = new Date()): string {
  const fecha = inicio(new Date(enviada * 1000));
  fecha.setDate(fecha.getDate() + DIAS_PAPELERA);
  return `Se eliminará el ${corta(fecha, hoy)}`;
}
