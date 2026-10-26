<script lang="ts" module>
  // Iconos de trazo en SVG en línea (decisión 003): sin fuentes de iconos.
  const trazos = {
    fotos:
      "M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1Zm-1 11 5-5 4 4 3-3 6 6M15.5 9.5h.01",
    explorar: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Zm3.5-12.5-2 5-5 2 2-5 5-2Z",
    albumes: "M4 7h16v12H4zM7 4h10M6 7V5.5M18 7V5.5",
    etiquetas: "M3 12V4h8l10 10-8 8L3 12Zm5-4.5h.01",
    favoritos:
      "M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10Z",
    papelera: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
    buscar: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14Zm9 3-4-4",
    importar: "M12 4v11m0 0-4-4m4 4 4-4M5 20h14",
    cerrar: "M6 6l12 12M18 6 6 18",
    anterior: "M15 5l-7 7 7 7",
    siguiente: "M9 5l7 7-7 7",
    info: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Zm0-10v6m0-9.5h.01",
  } as const;

  export type NombreIcono = keyof typeof trazos;
</script>

<script lang="ts">
  let { nombre, tamano = 20 }: { nombre: NombreIcono; tamano?: number } = $props();
</script>

<svg
  width={tamano}
  height={tamano}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.75"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={trazos[nombre]} />
</svg>
