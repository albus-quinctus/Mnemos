// Textos y conversiones de la vista Duplicados (spec 010).
import type { FotoCuadricula } from "$lib/biblioteca";
import type { CopiaDuplicada, GrupoDuplicado, ResumenDuplicados } from "$lib/biblioteca";
import { nombreDia } from "$lib/cuadricula/disposicion";
import { tamano } from "$lib/visor/formato";

function dosCifras(n: number): string {
  return String(n).padStart(2, "0");
}

/** Segundos Unix → "2024-08-17" en hora local, como agrupa la cuadrícula. */
export function diaLocal(segundos: number): string {
  const d = new Date(segundos * 1000);
  return `${d.getFullYear()}-${dosCifras(d.getMonth() + 1)}-${dosCifras(d.getDate())}`;
}

/** "Originales/2024/08/IMG.jpg" → carpeta "Originales/2024/08/" y nombre "IMG.jpg". */
export function separarRuta(ruta: string): { carpeta: string; nombre: string } {
  const corte = ruta.lastIndexOf("/") + 1;
  return { carpeta: ruta.slice(0, corte), nombre: ruta.slice(corte) };
}

/** "3 grupos · 4 copias de más · 13,3 MB". */
export function textoResumen(r: ResumenDuplicados): string {
  const grupos = r.groups === 1 ? "1 grupo" : `${r.groups} grupos`;
  const copias = r.extra_copies === 1 ? "1 copia de más" : `${r.extra_copies} copias de más`;
  return `${grupos} · ${copias} · ${tamano(r.extra_bytes)}`;
}

/** Fecha de una copia: "Sáb, 17 ago 2024". */
export function fechaCopia(c: CopiaDuplicada, hoy = new Date()): string {
  return nombreDia(diaLocal(c.date), hoy);
}

/** Las copias de un grupo como fotos para el visor. */
export function fotosDelGrupo(g: GrupoDuplicado): FotoCuadricula[] {
  return g.copies.map((c) => {
    const dia = diaLocal(c.date);
    return {
      id: c.id,
      name: separarRuta(c.rel_path).nombre,
      month: dia.slice(0, 7),
      date: c.date,
      width: null,
      height: null,
      thumbnail_failed: false,
      favorite: c.favorite,
      archived: c.archived,
    };
  });
}
