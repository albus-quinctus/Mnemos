// Estado del campo de búsqueda (spec 003). `texto` es lo que hay escrito y
// `consulta` lo que se está buscando: llega 200 ms después de la última tecla.

const ESPERA = 200;

export const busqueda = $state({
  texto: "",
  consulta: "",
  // Sin biblioteca abierta no hay nada que buscar.
  disponible: false,
});

let temporizador: ReturnType<typeof setTimeout> | undefined;

export function escribir(texto: string) {
  busqueda.texto = texto;
  clearTimeout(temporizador);
  temporizador = setTimeout(() => (busqueda.consulta = texto.trim()), ESPERA);
}

/** Vuelve a la biblioteca entera. */
export function limpiarBusqueda() {
  clearTimeout(temporizador);
  busqueda.texto = "";
  busqueda.consulta = "";
}
