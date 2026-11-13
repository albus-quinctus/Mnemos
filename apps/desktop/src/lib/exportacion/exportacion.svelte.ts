// Exportar la selección (spec 017). El núcleo exporta foto a foto en un hilo
// aparte y avisa con el evento `exportacion`.

import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  cancelarExportacion as cancelar,
  exportarVarias as empezar,
  type EventoExportacion,
  type TamanoExportacion,
} from "../biblioteca";

export const exportacion = $state<{ actual: EventoExportacion | null }>({ actual: null });

// Última carpeta a la que se exportó en esta sesión, desde la barra o desde
// el visor: el selector la propone.
export const destino: { ultima: string | undefined } = { ultima: undefined };

let escuchando = false;

// Recibe los avances del núcleo. Se llama una vez al arrancar la aplicación.
export function escucharExportacion(): void {
  if (escuchando) return;
  escuchando = true;
  listen<EventoExportacion>("exportacion", (e) => {
    exportacion.actual = e.payload;
  }).catch(() => (escuchando = false));
}

export const enCurso = () => exportacion.actual?.estado === "en_curso";

function tituloSelector(n: number): string {
  return n === 1
    ? "Elige dónde guardar la foto"
    : `Elige dónde guardar las ${n.toLocaleString("es")} fotos`;
}

// Pide la carpeta y exporta `ids` a ella. Si se cancela el selector, nada.
export async function exportarVarias(ids: number[], tamano: TamanoExportacion): Promise<void> {
  if (ids.length === 0 || enCurso()) return;
  const carpeta = await open({
    directory: true,
    multiple: false,
    title: tituloSelector(ids.length),
    defaultPath: destino.ultima,
  });
  if (typeof carpeta !== "string") return;
  destino.ultima = carpeta;
  exportacion.actual = {
    estado: "en_curso",
    total: ids.length,
    hechas: 0,
    guardadas: 0,
    carpeta,
    ruta: null,
    saltadas: [],
    mensaje: null,
  };
  try {
    await empezar(ids, carpeta, tamano);
  } catch (e) {
    const texto = String(e);
    exportacion.actual = {
      ...exportacion.actual,
      estado: "detenida",
      mensaje: texto.charAt(0).toUpperCase() + texto.slice(1),
    };
  }
}

export function cancelarExportacion(): Promise<void> {
  return cancelar();
}

// La tarjeta se cierra a mano, pero no mientras exporta.
export function cerrarExportacion(): void {
  if (!enCurso()) exportacion.actual = null;
}
