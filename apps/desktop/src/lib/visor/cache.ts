// Caché del visor (spec 002): guarda como mucho unas pocas imágenes completas
// en memoria para que pasar a la anterior o a la siguiente sea instantáneo.
// Nunca se escribe nada en disco.

export class CacheImagenes<T> {
  private readonly entradas = new Map<number, T>();

  constructor(
    private readonly maximo: number,
    private readonly cargar: (id: number) => T,
    private readonly soltar: (valor: T) => void = () => {},
  ) {}

  /** Devuelve la imagen de `id`, cargándola si hace falta, y la marca como reciente. */
  obtener(id: number): T {
    let valor = this.entradas.get(id);
    if (valor === undefined) {
      valor = this.cargar(id);
    } else {
      this.entradas.delete(id);
    }
    this.entradas.set(id, valor);
    this.recortar(new Set([id]));
    return valor;
  }

  /** Carga estas imágenes (la actual y sus vecinas) y suelta las más antiguas. */
  precargar(ids: number[]): void {
    for (const id of [...ids].reverse()) {
      if (!this.entradas.has(id)) this.entradas.set(id, this.cargar(id));
      else {
        const valor = this.entradas.get(id)!;
        this.entradas.delete(id);
        this.entradas.set(id, valor);
      }
    }
    this.recortar(new Set(ids));
  }

  get ids(): number[] {
    return [...this.entradas.keys()];
  }

  vaciar(): void {
    for (const valor of this.entradas.values()) this.soltar(valor);
    this.entradas.clear();
  }

  private recortar(protegidas: Set<number>): void {
    // La más antigua es la primera del Map.
    for (const [id, valor] of this.entradas) {
      if (this.entradas.size <= this.maximo) break;
      if (protegidas.has(id)) continue;
      this.entradas.delete(id);
      this.soltar(valor);
    }
  }
}

/** Vecinas que se precargan: la actual primero, luego la siguiente y la anterior. */
export function vecinas(indice: number, total: number): number[] {
  return [indice, indice + 1, indice - 1].filter((i) => i >= 0 && i < total);
}
