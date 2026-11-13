<script lang="ts" module>
  // Iconos de trazo en SVG en línea (decisión 003): sin fuentes de iconos.
  const trazos = {
    fotos:
      "M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1Zm-1 11 5-5 4 4 3-3 6 6M15.5 9.5h.01",
    explorar: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Zm3.5-12.5-2 5-5 2 2-5 5-2Z",
    albumes: "M4 7h16v12H4zM7 4h10M6 7V5.5M18 7V5.5",
    etiquetas: "M3 12V4h8l10 10-8 8L3 12Zm5-4.5h.01",
    papelera: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
    buscar: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14Zm9 3-4-4",
    importar: "M12 4v11m0 0-4-4m4 4 4-4M5 20h14",
    exportar: "M12 15V4m0 0-4 4m4-4 4 4M5 14v6h14v-6",
    cerrar: "M6 6l12 12M18 6 6 18",
    anterior: "M15 5l-7 7 7 7",
    siguiente: "M9 5l7 7-7 7",
    duplicados: "M8 8h11v11H8zM5 16V5h11",
    info: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Zm0-10v6m0-9.5h.01",
    estrella:
      "m12 3.5 2.6 5.3 5.9.9-4.3 4.1 1 5.8L12 16.8l-5.2 2.8 1-5.8-4.3-4.1 5.9-.9L12 3.5Z",
    archivo: "M3 5h18v4H3zM5 9v10h14V9M10 13h4",
    desarchivar: "M3 5h18v4H3zM5 9v10h14V9M12 17v-5m0 0-2.5 2.5M12 12l2.5 2.5",
    marca: "m7 12.5 3.2 3.2L17 9",
    editar: "M4 20h4L19 9l-4-4L4 16v4ZM13.5 6.5l4 4",
    girar: "M4 9a8 8 0 1 1 1.6 7.4M4 4v5h5",
    voltear: "M12 3v18M9 7 4 17h5zM15 7l5 10h-5z",
    vertical: "M7 4h10v16H7zM19 9l2 2-2 2M5 9l-2 2 2 2",
    desplegar: "m6 9 6 6 6-6",
    acceso: "M3 5h18v11H3zM9 20h6M12 16v4M12 8v5M9.5 10.5h5",
    ajustes:
      "M18.5 9.3 20.8 10v4l-2.3.7 1.1 2.1-2.8 2.8-2.1-1.1-.7 2.3h-4l-.7-2.3-2.1 1.1-2.8-2.8 1.1-2.1-2.3-.7v-4l2.3-.7-1.1-2.1 2.8-2.8 2.1 1.1.7-2.3h4l.7 2.3 2.1-1.1 2.8 2.8-1.1 2.1ZM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z",
    pantalla: "M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5",
  } as const;

  export type NombreIcono = keyof typeof trazos;
</script>

<script lang="ts">
  let {
    nombre,
    tamano = 20,
    grosor = 1.75,
    relleno = false,
  }: { nombre: NombreIcono; tamano?: number; grosor?: number; relleno?: boolean } = $props();
</script>

<svg
  width={tamano}
  height={tamano}
  viewBox="0 0 24 24"
  fill={relleno ? "currentColor" : "none"}
  stroke="currentColor"
  stroke-width={grosor}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={trazos[nombre]} />
</svg>
