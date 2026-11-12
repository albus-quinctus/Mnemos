# Arquitectura

> Borrador. Las decisiones firmes se registran en `docs/decisiones/`.

## Capas
| Capa | Ubicación | Responsabilidad |
|---|---|---|
| Interfaz | `apps/desktop/src` (Svelte 5) | Cuadrícula, visor, álbumes, editor |
| Puente | `apps/desktop/src-tauri` | Comandos Tauri, permisos mínimos |
| Núcleo | `crates/mnemos-core` | Biblioteca, base de datos, indexado, búsqueda |
| Imagen | `crates/mnemos-imaging` | Miniaturas, EXIF, aplicación de recetas de edición |
| CLI | `crates/mnemos-cli` | Reindexar, verificar integridad, exportar |

## Formato de la biblioteca en el disco
```
DISCO/
├── MNEMOS-windows.exe, MNEMOS-macos.app, MNEMOS-linux.AppImage
└── Biblioteca/
    ├── Originales/AAAA/MM/...        (nunca se modifican)
    └── .mnemos/
        ├── mnemos.db                 (SQLite, rutas relativas)
        ├── miniaturas/v1/<ab>/       (<huella>.jpg y, editadas, <huella>-e<versión>.jpg)
        ├── ediciones/<ab>/           (<huella>.json: receta con versiones)
        ├── copias/
        └── tmp/                      (copias a medias durante la importación)
```

## Reglas
- Solo rutas relativas a `Biblioteca/`.
- Edición no destructiva: la imagen editada se genera a partir del original y la receta.
- Sistema de archivos del disco: exFAT (spec 005; guía en `docs/guias/pasar-el-disco-a-exfat.md`).

## Edición no destructiva (spec 009)
- **Receta:** una por contenido, en `.mnemos/ediciones/<2 primeros>/<huella>.json`. Guarda todas las versiones (giro, volteo y recorte en fracciones) y cuál se ve (`current`; `null` es el original). Se escribe en `.mnemos/tmp/` y se mueve a su sitio, así que un corte deja la anterior. La receta en disco manda: `photos.edit_version` es solo una caché que el indexado recalcula, por eso una biblioteca copiada o con la base borrada conserva sus ediciones.
- **Orden de la geometría:** orientación EXIF, giro (horario), volteo horizontal y recorte (fracciones de la imagen ya girada y volteada). `mnemos-imaging::recipe` lo aplica igual a la miniatura, al visor y a las copias. Exportar (spec 008) aún saca la foto sin editar.
- **Dónde se ve:** la cola de miniaturas genera la editada (`-e<versión>`); el protocolo `mnemos://foto/<id>` aplica la receta en memoria (ETag con la versión) y `mnemos://original/<id>` da la foto sin editar para el editor. La cuadrícula recibe `edit_version` y el tamaño editado de cada foto.
- **Interfaz:** estado puro del editor en `lib/editor/editor.ts`, editor en `lib/editor/Editor.svelte` y sección «Ediciones» en el panel de detalles del visor.
- **Guardar como copia:** único caso en que se escribe en `Originales/`, y siempre un archivo nuevo `nombre (N).ext` junto al original; nunca se sobrescribe ni se borra nada.
- **Prueba de punta a punta:** `crates/mnemos-core/tests/ediciones_originales_intactos.rs` comprueba huella BLAKE3 y fecha de modificación de cada original tras editar, cambiar de versión, volver al original, copiar, generar miniaturas y reindexar, y que las ediciones sobreviven a mover la biblioteca.
