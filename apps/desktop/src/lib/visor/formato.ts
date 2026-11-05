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
  manual: "Puesta a mano",
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

// Sección Cámara del panel (spec 011).

const numero = (valor: number, decimales = 1) =>
  valor.toLocaleString("es", { maximumFractionDigits: decimales });

/** 0.004 → "1/250 s"; 2 → "2 s"; 0.5 → "1/2 s". */
export function exposicion(segundos: number): string {
  if (segundos >= 1) return `${numero(segundos)} s`;
  return `1/${Math.round(1 / segundos)} s`;
}

/** 2.8 → "ƒ/2,8". */
export function apertura(f: number): string {
  return `ƒ/${numero(f)}`;
}

/** 4.25 y 26 → "4,3 mm (26 mm en 35 mm)". */
export function focal(mm: number | null, equivalente: number | null): string | null {
  if (mm === null) return equivalente === null ? null : `${equivalente} mm en 35 mm`;
  const real = `${numero(mm)} mm`;
  return equivalente && equivalente !== Math.round(mm)
    ? `${real} (${equivalente} mm en 35 mm)`
    : real;
}

/** [41.3851, -2.1734] → "41,38510° N, 2,17340° O". */
export function coordenadas([lat, lon]: [number, number]): string {
  const grados = (v: number) =>
    `${Math.abs(v).toLocaleString("es", { minimumFractionDigits: 5, maximumFractionDigits: 5 })}°`;
  return `${grados(lat)} ${lat < 0 ? "S" : "N"}, ${grados(lon)} ${lon < 0 ? "O" : "E"}`;
}

// Formulario de fecha a mano.

const dos = (n: number) => String(n).padStart(2, "0");

/** Segundos Unix → día y hora locales para los campos del formulario. */
export function camposFecha(segundos: number): { dia: string; hora: string } {
  const d = new Date(segundos * 1000);
  return {
    dia: `${d.getFullYear()}-${dos(d.getMonth() + 1)}-${dos(d.getDate())}`,
    hora: `${dos(d.getHours())}:${dos(d.getMinutes())}:${dos(d.getSeconds())}`,
  };
}

/** Día y hora del formulario → "AAAA-MM-DD hh:mm:ss", o null si falta algo. */
export function fechaManual(dia: string, hora: string): string | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(dia) || !/^\d{2}:\d{2}(:\d{2})?$/.test(hora)) return null;
  return `${dia} ${hora.length === 5 ? `${hora}:00` : hora}`;
}
