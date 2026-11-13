// Etiquetas sobre la selección (spec 016): el desplegable «Etiquetar» de la
// barra y «Quitar de «X»» dentro de una etiqueta. Cada cambio va al núcleo en
// una transacción y el aviso permite deshacerlo.
import { ponerEtiqueta, quitarEtiqueta } from "$lib/biblioteca";
import { avisarCambio } from "$lib/cambios.svelte";
import { ejecutar, fotosTexto, mostrarAviso, terminar } from "$lib/seleccion/acciones.svelte";
import { limpiarSeleccion, seleccion } from "$lib/seleccion/seleccion.svelte";
import { resumenCambios, type CambioEtiqueta } from "./etiquetas";
import { buscarEtiqueta, cargarEtiquetas } from "./estado.svelte";

/** La etiqueta abierta, donde la barra ofrece «Quitar de «X»». */
export function etiquetaAbierta(): number | null {
  const ambito = seleccion.ambito;
  if (!("vista" in ambito)) return null;
  const vista = ambito.vista;
  return typeof vista === "object" && "etiqueta" in vista ? vista.etiqueta : null;
}

export function nombreEtiquetaAbierta(): string {
  const id = etiquetaAbierta();
  return id === null ? "" : (buscarEtiqueta(id)?.name ?? "la etiqueta");
}

/** Dentro de una etiqueta: se la quita a la selección; las fotos se quedan. */
export function quitarSeleccionDeEtiqueta() {
  const id = etiquetaAbierta();
  if (id === null) return;
  const nombre = nombreEtiquetaAbierta();
  return ejecutar(async () => {
    const cambiadas = await quitarEtiqueta([...seleccion.ids], id);
    terminar(`${fotosTexto(cambiadas.length, "quitada")} de «${nombre}»`, () =>
      ponerEtiqueta(cambiadas, id),
    );
    cargarEtiquetas();
  });
}

/**
 * Al cerrar el desplegable: si algo cambió, quita la selección, avisa a las
 * cuadrículas y enseña el aviso con «Deshacer», que deshace los cambios del
 * último al primero.
 */
export function terminarEtiquetado(cambios: (CambioEtiqueta & { id: number })[]) {
  const texto = resumenCambios(cambios);
  if (!texto) return;
  limpiarSeleccion();
  avisarCambio();
  cargarEtiquetas();
  mostrarAviso({
    texto,
    deshacer: async () => {
      for (const c of [...cambios].reverse()) {
        if (c.fotos.length === 0) continue;
        if (c.puesta) await quitarEtiqueta(c.fotos, c.id);
        else await ponerEtiqueta(c.fotos, c.id);
      }
      avisarCambio();
      cargarEtiquetas();
    },
  });
}
