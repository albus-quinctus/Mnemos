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

## Papelera (spec 015)
- **Enviar a la Papelera** solo pone la fecha en `photos.trashed_at`; el archivo no se mueve. Restaurar la quita. Todas las vistas, la búsqueda, la selección, Duplicados, álbumes y etiquetas filtran `trashed_at IS NULL`; la vista Papelera (`View::Trash`) ordena por la fecha de envío.
- **Eliminar para siempre** es la única excepción a la regla 1. Solo `trash::delete_original` borra originales, y solo los llama `delete_permanently`, a mano (diálogo de confirmación con el foco en Cancelar) o desde el vaciado automático. Salvaguardas: la foto tiene que estar en la Papelera; archivo normal dentro de la biblioteca, sin seguir enlaces; tamaño y fecha iguales a los de la base; si `Originales/` deja de estar (disco desconectado), se para. Después se borran la fila, su búsqueda, sus álbumes y etiquetas, y la receta y las miniaturas si ninguna otra foto comparte la huella.
- **Vaciado automático:** al abrir la biblioteca y cada hora se elimina lo que lleva 30 días o más. La última apertura se guarda en `library_meta` y nunca retrocede; si el reloj va más de 60 días por delante, no se borra nada hasta que el usuario lo confirme.
- **Segundo plano:** los comandos Tauri que borran usan su propia conexión, uno a la vez, y avisan con eventos `papelera`. Interfaz en `lib/papelera/`.
- **Pruebas:** `tests/solo_la_papelera_borra.rs` falla si otro código del núcleo borra o mueve archivos, y `tests/originales_intactos.rs` comprueba que enviar y restaurar no tocan nada y que eliminar o vaciar borra solo los originales de la Papelera.

## Etiquetas (spec 016)
- **Datos:** `tags(id, name UNIQUE COLLATE NOCASE)` y `photo_tags(photo_id, tag_id)`; la migración 0013 añade el índice por etiqueta. Las etiquetas solo viven en `.mnemos/mnemos.db`: nunca se escriben en los archivos de imagen.
- **Búsqueda:** la columna `tags` de `photo_search` guarda los nombres de las etiquetas de cada foto. Cada cambio (poner, quitar, renombrar, unir, borrar) la rehace para las fotos afectadas dentro de la misma transacción (`refresh_search`).
- **Núcleo:** `tags.rs`. Poner y quitar a varias fotos devuelven las que cambian, para «Deshacer». `View::Tag(id)` (`{"etiqueta": id}`) incluye las archivadas y deja fuera la Papelera. «Guardar como copia» copia las etiquetas (`copy_tags`).
- **Interfaz:** `lib/etiquetas/`: pantalla Etiquetas, cabecera de una etiqueta, «Etiquetar» en la barra de selección y la sección del panel Detalles del visor (tecla `T`).
- **Pruebas:** `tests/originales_intactos.rs` comprueba que ninguna acción de etiquetas toca los originales y que borrar una etiqueta no quita fotos.
