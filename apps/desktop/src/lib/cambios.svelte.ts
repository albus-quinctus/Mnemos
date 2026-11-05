// Avisa a las cuadrículas de que han cambiado datos de fotos que ya tienen
// cargadas (favoritos, archivo; spec 006). Cada una se vuelve a cargar sin
// perder su posición, igual que con `fechas.version`.
export const cambios = $state({ version: 0 });

export function avisarCambio() {
  cambios.version++;
}
