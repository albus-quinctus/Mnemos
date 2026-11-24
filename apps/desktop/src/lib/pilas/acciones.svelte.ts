// Apilar, desapilar y elegir la portada desde la barra de selección (spec 023),
// con el aviso de siempre.
import { apilar, desapilar, deshacerPila, usarComoPortada, type DeshacerPila } from "$lib/biblioteca";
import { avisarCambio } from "$lib/cambios.svelte";
import {
  avisos,
  fotosSeleccionadas,
  mostrarAviso,
  pilasSeleccionadas,
} from "$lib/seleccion/acciones.svelte";
import { limpiarSeleccion, seleccion } from "$lib/seleccion/seleccion.svelte";
import { textoFotos } from "./pilas.svelte";

async function hacer(accion: () => Promise<DeshacerPila>, texto: string) {
  avisos.ocupado = true;
  try {
    const antes = await accion();
    avisarCambio();
    limpiarSeleccion();
    mostrarAviso({
      texto,
      deshacer: async () => {
        await deshacerPila(antes);
        avisarCambio();
      },
    });
  } catch (e) {
    mostrarAviso({ texto: `No se ha podido completar: ${String(e)}`, error: true });
  } finally {
    avisos.ocupado = false;
  }
}

export function apilarSeleccion() {
  const n = fotosSeleccionadas();
  return hacer(() => apilar([...seleccion.ids]), `${textoFotos(n)} apiladas`);
}

export function desapilarSeleccion() {
  const pilas = pilasSeleccionadas();
  const texto = pilas.length === 1 ? "Pila deshecha" : `${pilas.length} pilas deshechas`;
  return hacer(() => desapilar(pilas), texto);
}

export function usarSeleccionComoPortada() {
  const [id] = seleccion.ids;
  return hacer(() => usarComoPortada(id), "Portada cambiada");
}
