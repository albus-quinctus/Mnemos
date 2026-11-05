// Avisa a las cuadrículas de que alguna fecha de captura ha cambiado a mano
// (spec 011). El visor sube `version` al cerrarse, y cada cuadrícula se vuelve
// a cargar sin perder su posición.
export const fechas = $state({ version: 0 });
