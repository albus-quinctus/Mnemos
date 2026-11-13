# 015 · Papelera

- Estado: aprobada (Paulo, 2026-11-12)
- Fase: F2

## Qué y por qué
Quiero quitar de mi biblioteca las fotos que sobran (repetidas, borrosas,
capturas) sin miedo a equivocarme: primero van a la Papelera, desde donde las
puedo recuperar durante 30 días, y solo después se borran del disco. También
quiero poder borrarlas para siempre en el momento, para liberar espacio en el
HDD.

Es la única funcionalidad de MNEMOS que borra archivos, así que necesita una
excepción a la regla 1 de `CLAUDE.md` (ver más abajo). Lo decidió Paulo el
2026-11-01 en la comparativa con Google Fotos: vaciado automático a los 30
días y borrado definitivo en cualquier momento.

## Excepción a la regla 1 de CLAUDE.md (aprobada el 2026-11-12)
La regla 1 decía: «Nunca modificar, mover ni borrar un original». Paulo
aprobó cambiarla el 2026-11-12 por:

> 1. **Nunca modificar, mover ni borrar un original** de la biblioteca. Las
>    ediciones son recetas JSON en `.mnemos/ediciones/`. **Única excepción:**
>    eliminar para siempre desde la Papelera (spec 015), a mano con
>    confirmación o al cumplirse los 30 días. Ningún otro código borra
>    originales.

Salvaguardas que la acompañan:

1. **Enviar a la Papelera no toca el archivo.** Solo pone la fecha en
   `photos.trashed_at`, como archivar. La foto sigue en `Originales/` en su
   sitio, con su nombre. Restaurar es borrar esa fecha.
2. **Una sola función borra originales:** `trash::delete_original` en
   `crates/mnemos-core/src/trash.rs`. Solo la llaman «Eliminar para siempre»
   y el vaciado (a mano o a los 30 días), y solo para fotos que ya están en la
   Papelera. Una prueba recorre el código fuente y falla si aparece
   `remove_file` sobre una ruta de `Originales/` en cualquier otro archivo que
   no sea una prueba.
3. **Nunca al importar, indexar, editar, exportar ni copiar.** Las pruebas de
   `originales_intactos.rs` y `ediciones_originales_intactos.rs` se amplían
   con la Papelera: enviar y restaurar no cambia huella, tamaño ni fecha, y
   ninguna otra acción borra nada.
4. **Comprobación antes de borrar:** el archivo se borra solo si su tamaño y
   su fecha de modificación coinciden con los de la base de datos. Si no
   coinciden (alguien lo cambió o lo sustituyó fuera de la app), no se borra,
   la foto se queda en la Papelera y sale un aviso.
5. **Confirmación clara:** «Eliminar para siempre» y «Vaciar papelera» abren
   un diálogo con el número de fotos y el texto «Se borrarán del disco y no se
   podrán recuperar». El foco empieza en «Cancelar»; `Intro` no confirma, hay
   que pulsar el botón rojo.
6. **Reloj adelantado:** el vaciado automático no se hace si la fecha del
   equipo va más de 60 días por delante de la última vez que se abrió la
   biblioteca (un equipo con la hora mal puesta podría vaciarla entera). En
   ese caso se pregunta, con el mismo diálogo.
7. Solo se borra lo que está dentro de la raíz de la biblioteca (rutas
   relativas, regla 2); nunca se sigue un enlace simbólico hacia fuera.

## Comportamiento

### Enviar a la Papelera
- Desde la barra de selección (botón «Papelera», después de Archivar), desde
  el visor (botón de papelera junto a archivar) o con la tecla `Supr`
  (`Suprimir`, y `Retroceso` en macOS) con selección o en el visor.
- No pide confirmación porque se puede deshacer: aparece el aviso de 8 s
  «3 fotos movidas a la papelera · Deshacer». En el visor se pasa a la foto
  siguiente.
- Se puede enviar desde Fotos, Favoritos, Archivo, los resultados de una
  búsqueda, Duplicados y (cuando existan) álbumes y etiquetas.
- La foto deja de salir en todas partes: Fotos, Favoritos, Archivo,
  búsqueda, barra de meses, Duplicados, álbumes y etiquetas. Solo sale en la
  Papelera. Conserva su favorito, su archivo, sus álbumes, sus etiquetas y su
  edición, que vuelven al restaurarla.

### Vista Papelera
- La entrada «Papelera» del menú lateral se activa.
- Arriba, una franja con el texto «Las fotos de la papelera se eliminan para
  siempre a los 30 días.» y el botón «Vaciar papelera».
- La cuadrícula ordena por la fecha en que se envió cada foto, de la más
  reciente a la más antigua. Cada día lleva su cabecera con la fecha de envío
  y cuántos días le quedan: «Enviadas el 10 nov · se eliminan en 28 días».
  Con 1 día o menos: «se eliminan mañana».
- Vacía: «La papelera está vacía. Las fotos que envíes aquí se eliminan para
  siempre a los 30 días.»
- En la Papelera, la barra de selección tiene «Restaurar» y «Eliminar para
  siempre» (en rojo). `Ctrl` + `A` selecciona todo, como en las demás vistas.
- El visor de la Papelera enseña la foto con dos botones, «Restaurar» y
  «Eliminar para siempre», y en el panel de detalles «Se eliminará el 12 dic».
  No se puede editar, marcar como favorita ni archivar desde ahí.

### Restaurar
- Devuelve la foto a donde estaba (Fotos o Archivo, con su favorito).
- Aviso de 8 s «3 fotos restauradas · Deshacer».

### Eliminar para siempre
- Desde la barra de selección o el visor de la Papelera, y «Vaciar papelera»
  para todas.
- Diálogo: «¿Eliminar 3 fotos para siempre?» / «Se borrarán del disco y no se
  podrán recuperar.» / [Cancelar] [Eliminar para siempre].
- Por cada foto: se comprueba el archivo (salvaguarda 4), se borra el archivo
  de `Originales/`, y en una transacción se borra su fila y lo que cuelga de
  ella (álbumes, etiquetas, entrada de búsqueda). Si ninguna otra foto
  comparte su huella, también se borran su receta de `.mnemos/ediciones/` y
  sus miniaturas.
- Las copias hechas con «Guardar como copia» (spec 009) son fotos aparte: no
  se borran con el original.
- Con muchas fotos sale el progreso («Eliminando 120 de 800…») y la interfaz
  no se bloquea. Al terminar: «800 fotos eliminadas».
- Sin «Deshacer»: es definitivo y así lo dice el diálogo.

### Vaciado automático a los 30 días
- Al abrir la biblioteca, en segundo plano, se eliminan las fotos que llevan
  30 días o más en la Papelera (con las mismas comprobaciones). Si la app se
  queda abierta, se repite cada hora.
- Si se ha eliminado alguna, aviso: «Se eliminaron 4 fotos que llevaban 30
  días en la papelera».
- La biblioteca guarda la fecha de la última apertura para la salvaguarda 6.

### Línea de órdenes
- `mnemos papelera` lista lo que hay y cuándo se elimina cada foto.
- `mnemos papelera vaciar` pide escribir «sí» antes de borrar (o
  `--confirmar` en scripts). No hay orden para enviar fotos a la Papelera:
  eso se hace en la app.

### Datos
- Migración `0014_papelera.sql`:
  - `photos.trashed_at` (segundos Unix, UTC, o NULL).
  - Rehace los índices parciales de la línea de tiempo (migración 0007) para
    excluir las fotos en la Papelera, y añade `photos_papelera` sobre
    `(trashed_at DESC, id DESC) WHERE trashed_at IS NOT NULL`.
  - Tabla `library_meta` (clave y valor) con `ultima_apertura`.
- `View` gana `Papelera`; todas las vistas, la búsqueda, los duplicados, los
  álbumes y las etiquetas filtran `trashed_at IS NULL`. Las specs 014
  (álbumes) y 016 (etiquetas) tienen que respetarlo: la que se fusione después
  añade el filtro a la otra.
- Enviar y restaurar van en una sola transacción, como archivar.

## Casos límite
- **Disco desconectado a mitad de un vaciado:** las fotos ya borradas están
  borradas; las demás siguen en la Papelera. El orden (archivo primero, fila
  después) hace que, si se corta entre los dos pasos, quede una fila sin
  archivo: sale como desaparecida en la Papelera y el siguiente vaciado la
  quita.
- **Disco de solo lectura o archivo bloqueado (Windows):** no se borra, la
  foto se queda en la Papelera y el aviso dice cuántas no se pudieron borrar.
- **Foto desaparecida del disco:** se puede enviar a la Papelera, restaurar y
  eliminar; eliminarla solo borra su fila.
- **Importar una foto que está en la Papelera:** la importación ya salta las
  fotos cuya huella está en la biblioteca, así que no entra otra vez. El
  informe la cuenta aparte: «2 ya estaban en la papelera». Una foto eliminada
  para siempre sí se puede volver a importar.
- **Reindexar:** una foto en la Papelera sigue en ella (misma ruta o misma
  huella, como con favoritos y archivo).
- **Duplicados exactos:** enviar una copia a la Papelera no afecta a las
  demás; la receta compartida y las miniaturas no se borran mientras quede
  otra foto con esa huella.
- **Varias fotos con el mismo nombre en carpetas distintas:** cada una es su
  fila y su ruta; se borra exactamente la ruta de la fila.
- **Biblioteca abierta en dos equipos con relojes distintos:** cuenta la
  fecha guardada al enviar; la salvaguarda 6 cubre el reloj adelantado.
- **RAW + JPEG de la misma foto:** son dos fotos; se envían por separado.
- **Rutas con caracteres especiales en Windows, macOS y Linux:** la interfaz
  solo maneja ids y el núcleo usa rutas relativas.

## Criterios de aceptación
- [ ] Enviar a la Papelera y restaurar no cambian huella, tamaño ni fecha del
  original (prueba automática).
- [ ] Ninguna acción que no sea eliminar desde la Papelera borra un archivo:
  importar, indexar, editar, copiar, exportar, archivar, favoritos (prueba
  automática ampliando `originales_intactos.rs`).
- [ ] Solo `trash.rs` contiene código que borra originales (prueba que
  recorre el código fuente).
- [ ] Eliminar para siempre borra exactamente esos archivos y filas y nada
  más; un archivo cambiado fuera de la app no se borra (prueba automática).
- [ ] El vaciado automático borra solo lo que lleva 30 días o más, y no hace
  nada con el reloj adelantado más de 60 días (prueba automática con reloj
  simulado).
- [ ] Las fotos en la Papelera no salen en Fotos, Favoritos, Archivo,
  búsqueda, barra de meses ni Duplicados (prueba automática).
- [ ] Restaurar devuelve favorito, archivo y edición (prueba automática;
  favorito y archivo ya probados en `trash.rs`).
- [x] Enviar o restaurar 10 000 fotos tarda menos de 1 s (30 ms,
  `docs/rendimiento.md`).
- [x] Capturas de la Papelera, la barra de selección y el diálogo aprobadas
  por Paulo antes de cerrar la interfaz.
- [ ] Los originales no se modifican salvo al eliminar para siempre (prueba
  automática).

## Fuera de alcance
- Mandar los archivos a la papelera del sistema (Windows, macOS, Linux):
  cada sistema la gestiona distinto en un disco externo exFAT y la biblioteca
  dejaría de ser portátil. MNEMOS lleva su propia Papelera dentro de la base
  de datos.
- Cambiar el plazo de 30 días (podría ir en Ajustes, spec 013, más adelante).
- Borrar duplicados de golpe desde la vista Duplicados («Quedarse con una»):
  spec aparte que usará esta Papelera.

## Tareas (≤1 h cada una)
1. [x] Aprobación de Paulo de la excepción a la regla 1 y cambio de la regla
   en `CLAUDE.md`.
2. [x] Migración `0014_papelera.sql` (`trashed_at`, índices y tabla
   `library_meta` para la última apertura); filtros en las vistas, la
   búsqueda, la barra de meses, la selección y los duplicados, con pruebas.
3. [x] Núcleo: `send_to_trash` y `restore_from_trash` sobre un `Scope`, en una
   transacción, y la vista Papelera ordenada por fecha de envío (con su
   cursor y su selección), con pruebas y medida con 10 000 fotos.
4. [ ] Núcleo: `trash::delete_original` y `delete_permanently` (comprobación
   de tamaño y fecha, recetas y miniaturas sin otra foto con la huella), con
   pruebas y la prueba que recorre el código fuente.
5. [ ] Núcleo: vaciado a los 30 días con reloj inyectable y salvaguarda del
   reloj adelantado; informe de importación con «ya estaban en la papelera».
6. [ ] CLI `mnemos papelera` y `mnemos papelera vaciar`.
7. [ ] Comandos de Tauri (`enviar_a_papelera`, `restaurar`,
   `eliminar_para_siempre`, `vaciar_papelera`, vaciado al abrir) con
   progreso, y envoltorios en `biblioteca.ts`.
8. [x] Capturas del diseño para Paulo (aprobadas el 2026-11-12;
   `capturas/015-*.png`).
9. [ ] Interfaz: botón y tecla `Supr` en la barra de selección y en el visor,
   con aviso y «Deshacer».
10. [ ] Interfaz: vista Papelera (franja, cabeceras con los días que quedan,
    vacía), barra con Restaurar y Eliminar para siempre, visor de la Papelera
    y diálogo de confirmación.
11. [ ] Prueba de punta a punta ampliando `originales_intactos.rs` y
    `docs/arquitectura.md`; actualizar `docs/progreso.md`.
