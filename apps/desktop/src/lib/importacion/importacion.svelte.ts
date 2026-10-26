import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

export type EventoImportacion = {
  estado: "en_curso" | "terminada" | "cancelada" | "fallida";
  total: number;
  procesadas: number;
  copiadas: number;
  duplicadas: number;
  errores: number;
  mensaje: string | null;
};

// La importación que se muestra. `version` sube al terminar con fotos nuevas,
// para que la cuadrícula se vuelva a cargar.
export const importacion = $state<{ actual: EventoImportacion | null; version: number }>({
  actual: null,
  version: 0,
});

let escuchando = false;

// Recibe los avances del núcleo. Se llama una vez al arrancar la aplicación.
export function escucharImportacion(): void {
  if (escuchando) return;
  escuchando = true;
  listen<EventoImportacion>("importacion", (e) => {
    importacion.actual = e.payload;
    if (e.payload.estado !== "en_curso" && e.payload.copiadas > 0) importacion.version++;
  }).catch(() => (escuchando = false));
}

// Elige una carpeta y la importa a la biblioteca abierta.
export async function importarCarpeta(): Promise<void> {
  const origen = await open({
    directory: true,
    multiple: false,
    title: "Elige la carpeta con las fotos a importar",
  });
  if (typeof origen !== "string") return;
  importacion.actual = {
    estado: "en_curso",
    total: 0,
    procesadas: 0,
    copiadas: 0,
    duplicadas: 0,
    errores: 0,
    mensaje: null,
  };
  try {
    await invoke("importar", { origen });
  } catch (e) {
    importacion.actual = { ...importacion.actual, estado: "fallida", mensaje: String(e) };
  }
}

export function cancelarImportacion(): Promise<void> {
  return invoke("cancelar_importacion");
}

export function cerrarImportacion(): void {
  importacion.actual = null;
}
