// Estado de las etiquetas (spec 016): la lista, la etiqueta que se está
// creando y las confirmaciones de borrar y unir. Solo cambian la base de
// datos; las fotos nunca se tocan.
import {
  borrarEtiqueta,
  crearEtiqueta,
  etiquetas as pedirEtiquetas,
  ponerEtiqueta,
  renombrarEtiqueta,
  unirEtiquetas,
  type Etiqueta,
} from "$lib/biblioteca";
import { avisarCambio } from "$lib/cambios.svelte";
import { abrirEtiqueta, irA, navegacion } from "$lib/navegacion.svelte";
import { mostrarAviso } from "$lib/seleccion/acciones.svelte";
import { ordenarEtiquetas } from "./etiquetas";

export type Confirmacion =
  | { tipo: "borrar"; etiqueta: Etiqueta }
  | { tipo: "unir"; desde: Etiqueta; en: Etiqueta };

export const estadoEtiquetas = $state({
  lista: [] as Etiqueta[],
  cargado: false,
  error: "",
  /** El campo «Nueva etiqueta» está abierto. */
  creando: false,
  confirmacion: null as Confirmacion | null,
});

/** Vuelve a pedir las etiquetas (al abrir la pantalla y tras cada cambio). */
export async function cargarEtiquetas() {
  try {
    estadoEtiquetas.lista = ordenarEtiquetas(await pedirEtiquetas());
    estadoEtiquetas.error = "";
  } catch (e) {
    estadoEtiquetas.error = String(e);
  } finally {
    estadoEtiquetas.cargado = true;
  }
}

/** La etiqueta `id` de la lista cargada. */
export function buscarEtiqueta(id: number): Etiqueta | undefined {
  return estadoEtiquetas.lista.find((e) => e.id === id);
}

/** Crea la etiqueta (o usa la que ya se llama así). Devuelve el error. */
export async function crear(nombre: string): Promise<string | null> {
  try {
    await crearEtiqueta(nombre);
    estadoEtiquetas.creando = false;
    await cargarEtiquetas();
    return null;
  } catch (e) {
    return String(e);
  }
}

/**
 * Cambia el nombre. Si ya es de otra etiqueta, pide confirmación para unirlas
 * en vez de cambiarlo. Devuelve el error, si lo hay.
 */
export async function renombrar(etiqueta: Etiqueta, nombre: string): Promise<string | null> {
  try {
    const r = await renombrarEtiqueta(etiqueta.id, nombre);
    if ("existe" in r) {
      estadoEtiquetas.confirmacion = { tipo: "unir", desde: etiqueta, en: r.existe };
    } else {
      await cargarEtiquetas();
      avisarCambio();
    }
    return null;
  } catch (e) {
    return String(e);
  }
}

export function pedirBorrar(etiqueta: Etiqueta) {
  estadoEtiquetas.confirmacion = { tipo: "borrar", etiqueta };
}

export function cancelarConfirmacion() {
  estadoEtiquetas.confirmacion = null;
}

/** Hace lo que pide la confirmación abierta (borrar o unir). */
export async function confirmar() {
  const c = estadoEtiquetas.confirmacion;
  if (!c) return;
  estadoEtiquetas.confirmacion = null;
  try {
    if (c.tipo === "unir") await unir(c.desde, c.en);
    else await borrar(c.etiqueta);
  } catch (e) {
    mostrarAviso({ texto: `No se ha podido completar: ${String(e)}`, error: true });
  }
}

async function unir(desde: Etiqueta, en: Etiqueta) {
  await unirEtiquetas(desde.id, en.id);
  if (navegacion.seccion === "etiquetas" && navegacion.etiqueta === desde.id) abrirEtiqueta(en.id);
  await cargarEtiquetas();
  avisarCambio();
  mostrarAviso({ texto: `«${desde.name}» se ha unido a «${en.name}»` });
}

async function borrar(etiqueta: Etiqueta) {
  const fotos = await borrarEtiqueta(etiqueta.id);
  if (navegacion.seccion === "etiquetas" && navegacion.etiqueta === etiqueta.id) irA("etiquetas");
  await cargarEtiquetas();
  avisarCambio();
  mostrarAviso({
    texto: `Etiqueta «${etiqueta.name}» borrada`,
    // Deshacer la recrea con el mismo nombre y las mismas fotos.
    deshacer: async () => {
      const nueva = await crearEtiqueta(etiqueta.name);
      if (fotos.length > 0) await ponerEtiqueta(fotos, nueva.id);
      await cargarEtiquetas();
      avisarCambio();
    },
  });
}
