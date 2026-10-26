// Textos del panel de detalles, en español.

/** 4 200 000 → "4,2 MB". */
export function tamano(bytes: number): string {
  const unidades = ["bytes", "KB", "MB", "GB"];
  let valor = bytes;
  let u = 0;
  while (valor >= 1000 && u < unidades.length - 1) {
    valor /= 1000;
    u++;
  }
  const decimales = u === 0 || valor >= 100 ? 0 : 1;
  return `${valor.toLocaleString("es", { maximumFractionDigits: decimales })} ${unidades[u]}`;
}

/** 4000 × 3000 → "4000 × 3000 · 12 MP". */
export function dimensiones(ancho: number | null, alto: number | null): string {
  if (!ancho || !alto) return "Desconocidas";
  const mp = (ancho * alto) / 1e6;
  const megapixeles = mp.toLocaleString("es", { maximumFractionDigits: mp < 10 ? 1 : 0 });
  return `${ancho} × ${alto} · ${megapixeles} MP`;
}

/** Fecha y hora locales: "26 oct 2026, 14:03". */
export function fecha(segundos: number): string {
  return new Date(segundos * 1000)
    .toLocaleString("es", {
      day: "numeric",
      month: "short",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    })
    .replace(/\./g, "");
}
