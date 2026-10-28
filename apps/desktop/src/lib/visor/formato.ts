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

const ORIGENES: Record<string, string> = {
  exif: "Según la cámara (EXIF)",
  nombre: "Según el nombre del archivo",
  takeout: "Según Google Fotos",
  nombre_dia: "Según el nombre del archivo, solo el día",
};

/** Fecha de captura y de dónde sale, o "Sin fecha de captura". */
export function fechaCaptura(
  segundos: number | null,
  fuente: string | null,
): { texto: string; origen: string | null } {
  if (segundos === null) return { texto: "Sin fecha de captura", origen: null };
  const texto =
    fuente === "nombre_dia"
      ? new Date(segundos * 1000)
          .toLocaleDateString("es", { day: "numeric", month: "short", year: "numeric" })
          .replace(/\./g, "")
      : fecha(segundos);
  return { texto, origen: (fuente && ORIGENES[fuente]) ?? null };
}
