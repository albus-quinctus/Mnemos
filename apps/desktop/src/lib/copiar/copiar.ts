// Copiar, descargar y «Guardar como…» (spec 020): atajos de teclado y textos
// compartidos por el visor y la cuadrícula.

type Tecla = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey">;

const ctrl = (e: Tecla) => e.ctrlKey || e.metaKey;

/** `Ctrl` + `C` (o `Cmd` + `C`): copiar la imagen. */
export function esTeclaCopiar(e: Tecla): boolean {
  return ctrl(e) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "c";
}

/** `Mayús` + `D`: descargar. */
export function esTeclaDescargar(e: Tecla): boolean {
  return e.shiftKey && !ctrl(e) && !e.altKey && e.key.toLowerCase() === "d";
}

/** `Ctrl` + `Mayús` + `S`: guardar como. */
export function esTeclaGuardarComo(e: Tecla): boolean {
  return ctrl(e) && e.shiftKey && !e.altKey && e.key.toLowerCase() === "s";
}

/**
 * Indica si `Ctrl` + `C` debe copiar texto y no la foto: el foco está en un
 * campo o hay texto seleccionado.
 */
export function copiaTexto(destino: EventTarget | null, seleccionado: string): boolean {
  const el = destino as { closest?: (s: string) => unknown } | null;
  if (el?.closest?.("input, textarea, [contenteditable]")) return true;
  return seleccionado.trim() !== "";
}

/** Nombre del archivo de una ruta, con `/` o `\`. */
export function nombreDe(ruta: string): string {
  return ruta.split(/[\\/]/).pop() ?? ruta;
}

/** Carpeta de una ruta, con `/` o `\`; vacía si no tiene. */
export function carpetaDe(ruta: string): string {
  const i = Math.max(ruta.lastIndexOf("/"), ruta.lastIndexOf("\\"));
  return i > 0 ? ruta.slice(0, i) : "";
}

/** Une carpeta y nombre con el separador que ya use la carpeta. */
export function unir(carpeta: string | undefined, nombre: string): string {
  if (!carpeta) return nombre;
  const sep = carpeta.includes("\\") && !carpeta.includes("/") ? "\\" : "/";
  return carpeta.endsWith(sep) ? carpeta + nombre : carpeta + sep + nombre;
}

/** Primera letra en mayúscula, para los errores que llegan del núcleo. */
export function mayuscula(texto: string): string {
  return texto.charAt(0).toUpperCase() + texto.slice(1);
}
