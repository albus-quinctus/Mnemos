// Pantalla completa (spec 013): F11 en cualquier pantalla y, en macOS, también
// Ctrl + Cmd + F, el atajo del sistema. Lo hace el núcleo (`set_fullscreen`).

import { alternarPantallaCompleta } from "../biblioteca";

type Tecla = Pick<
  KeyboardEvent,
  "key" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey"
>;

/** ¿Es el atajo de pantalla completa? */
export function esTeclaPantallaCompleta(e: Tecla): boolean {
  if (e.key === "F11")
    return !e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey;
  return (
    e.ctrlKey &&
    e.metaKey &&
    !e.altKey &&
    !e.shiftKey &&
    e.key.toLowerCase() === "f"
  );
}

/** Alterna la pantalla completa y devuelve el estado pedido, o `null` si no
 *  se pudo (fuera de Tauri, por ejemplo). */
export async function pantallaCompleta(): Promise<boolean | null> {
  try {
    return await alternarPantallaCompleta();
  } catch {
    return null;
  }
}
