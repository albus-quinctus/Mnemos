# 014 · Álbumes

- Estado: en curso
- Fase: F2

## Qué y por qué
Quiero juntar fotos en álbumes con nombre («Picos de Europa», «Boda de Ana»)
y abrirlos desde el menú lateral, igual que en Google Fotos. Los 13 álbumes
que vinieron del Takeout ya están en la base de datos (spec 007), pero hoy no
se pueden ver: la entrada «Álbumes» del menú está desactivada.

Un álbum es solo una lista de fotos en la base de datos. Crear, vaciar o
eliminar un álbum nunca toca los archivos (regla 1 de CLAUDE.md).

## Comportamiento

### Pantalla Álbumes
- La entrada «Álbumes» del menú lateral se activa y abre la pantalla.
- Cabecera: título «Álbumes», debajo «N álbumes», y a la derecha el botón
  «Nuevo álbum».
- Cuadrícula de tarjetas (unas 200 px de ancho, tantas columnas como quepan):
  portada cuadrada con esquinas redondeadas, nombre del álbum debajo (una línea,
  con «…» si no cabe) y «N fotos» en gris.
- **Portada:** la foto más reciente del álbum (por fecha de captura), con su
  edición si la tiene. Un álbum vacío muestra un recuadro gris con el icono de
  álbum.
- **Orden:** el álbum modificado más recientemente primero (al crearlo,
  renombrarlo o añadir o quitar fotos).
- Clic en una tarjeta: abre el álbum.
- Sin álbumes: «Aún no tienes álbumes. Selecciona fotos y pulsa “Añadir a
  álbum”, o crea uno con “Nuevo álbum”.»

### Dentro de un álbum
- La barra superior conserva la búsqueda; debajo, una cabecera con una flecha
  «Volver a Álbumes», el nombre del álbum en grande, y debajo
  «N fotos · 12 ago – 3 sept 2024» (rango de fechas de sus fotos).
- A la derecha, un botón de menú (⋮) con «Cambiar nombre» y «Eliminar álbum».
  Pulsar el nombre también lo deja editar en el sitio (como el título del día):
  `Intro` o salir del campo guarda, `Esc` cancela.
- Las fotos se ven con la misma cuadrícula por días, barra de meses, selección
  y visor que Fotos, de la más reciente a la más antigua.
- **Archivadas:** salen en el álbum, como en Google Fotos, con la caja pequeña
  en la miniatura que ya se usa en la búsqueda. Archivar una foto no la saca de
  sus álbumes.
- Álbum vacío: «Este álbum está vacío. Ve a Fotos, selecciona algunas y pulsa
  “Añadir a álbum”.»
- `Esc` sin selección ni búsqueda vuelve a la pantalla Álbumes.

### Crear y renombrar
- «Nuevo álbum» abre un diálogo pequeño: campo «Nombre del álbum» con el foco y
  los botones «Cancelar» y «Crear». El álbum nuevo se abre vacío.
- El nombre se guarda sin espacios al principio ni al final, y tiene entre 1 y
  100 caracteres. Con el campo vacío, «Crear» está desactivado.
- Puede haber dos álbumes con el mismo nombre (el Takeout ya los trae así).

### Añadir y quitar fotos
- **Barra de selección** (spec 006): botón nuevo «Añadir a álbum» (icono de
  álbum con +) junto a la estrella. Abre una lista desplegable:
  - arriba, «Nuevo álbum…», que pide el nombre y crea el álbum con las fotos
    seleccionadas;
  - debajo, los álbumes con su portada pequeña, en el mismo orden que la
    pantalla Álbumes, con un campo para filtrar por nombre si hay más de 8.
- Al elegir un álbum: aviso «N fotos añadidas a «Picos de Europa»» con el botón
  «Ver álbum» durante 8 s. Las fotos que ya estaban no se repiten ni cuentan.
- Funciona con cualquier selección, también con «Seleccionar todo» y por días
  sin páginas cargadas (usa el `Scope` de la spec 006).
- **Dentro de un álbum**, la barra de selección muestra además «Quitar del
  álbum». Las fotos salen del álbum (no de la biblioteca) y aparece el aviso
  «N fotos quitadas del álbum · Deshacer» durante 8 s.

### Eliminar un álbum
- Desde el menú ⋮ del álbum: diálogo «¿Eliminar el álbum «Picos de Europa»?
  Las fotos no se borran: siguen en tu biblioteca.» con «Cancelar» y
  «Eliminar álbum».
- Al eliminar se vuelve a la pantalla Álbumes. No hay deshacer: el diálogo hace
  de confirmación.

## Datos
- Tablas `albums` y `album_photos` de la migración 0001; el Takeout las rellena
  desde la 0008. Se reutilizan tal cual.
- **Migración 0012** (`0012_albumes.sql`):
  - `albums.updated_at INTEGER`, rellenada con `created_at` en los álbumes que
    ya existen, para ordenar la pantalla Álbumes.
  - Índice `album_photos (photo_id)`, para borrar fotos y para saber en qué
    álbumes está una foto sin recorrer la tabla.
- `album_photos.position` se sigue rellenando al añadir (al final), aunque la
  vista ordene por fecha; servirá para un orden manual más adelante.
- Núcleo: `View::Album(id)` en la línea de tiempo (meses, páginas, `Scope`),
  `albums()` con nombre, número de fotos, portada y rango de fechas,
  `create_album`, `rename_album`, `delete_album`, `add_to_album(id, scope)` y
  `remove_from_album(id, ids)`, cada uno en una transacción.

## Casos límite
- Disco desconectado a mitad de operación: cada cambio va en una transacción;
  si falla, el álbum queda como estaba y se ve el error en el aviso.
- Una foto desaparecida (spec 002) sigue en sus álbumes con el marcador «Sin
  vista previa».
- Si la foto de portada se quita, la portada pasa a la siguiente más reciente.
- Las fotos de la Papelera (spec 015) siguen en sus álbumes pero no se ven ni
  cuentan; si se restauran, vuelven a aparecer.
- Nombres con emojis, tildes, comillas o barras: solo se guardan en la base de
  datos, nunca se usan como nombre de carpeta o archivo.
- Añadir 10 000 fotos a la vez: una sola sentencia `INSERT … SELECT`, en menos
  de 1 s.
- Volver a importar el Takeout no debe resucitar un álbum eliminado ni volver a
  meter fotos quitadas a mano. Hoy `takeout::apply` añade el álbum siempre; con
  esta spec, el álbum se aplica una sola vez por foto, igual que el resto de
  metadatos (`takeout_at`). Una foto nueva del Takeout sí crea su álbum.

## Criterios de aceptación
- [ ] La entrada «Álbumes» del menú lateral abre la pantalla con los álbumes del
      Takeout, su portada y su número de fotos.
- [ ] Crear, renombrar y eliminar un álbum; eliminarlo no borra fotos.
- [ ] Añadir fotos desde la barra de selección (también con «Seleccionar todo»)
      y quitarlas dentro del álbum, con «Deshacer».
- [ ] Las archivadas salen en sus álbumes con la caja.
- [ ] El esquema sube a v12 y una biblioteca v11 se migra sin perder álbumes.
- [ ] Los originales no se modifican (prueba automática en
      `crates/mnemos-core/tests/originales_intactos.rs`).

## Fuera de alcance
- Elegir la portada a mano y ordenar las fotos del álbum a mano o de la más
  antigua a la más reciente.
- Álbumes compartidos, álbumes automáticos (personas, lugares) y carpetas de
  álbumes.
- Ver en qué álbumes está una foto desde el panel Detalles del visor.
- Buscar por nombre de álbum.
- Añadir a un álbum desde el visor (solo desde la barra de selección).

## Tareas (≤1 h cada una)
1. [x] Migración 0012 y núcleo: listar (portada, número, rango de fechas),
       crear, renombrar y eliminar álbumes; pruebas. `crates/mnemos-core/src/albums.rs`
       (`albums`, `album`, `create_album`, `rename_album`, `delete_album`).
2. [x] `View::Album` en meses, páginas y `Scope`; `add_to_album` y
       `remove_from_album`; álbum del Takeout una sola vez por foto; pruebas,
       también con 10 000 fotos. Añadir y quitar reciben ids, como Etiquetas
       (la interfaz los saca de `select_all` con el `Scope`). El álbum del
       Takeout se añade si su `.json` se aplica en esta importación (la foto
       suele venir en la carpeta del año y en la del álbum); sin `.json`, como
       antes.
3. [x] Comandos de Tauri y envoltorios en `biblioteca.ts`: `albumes`, `album`,
       `crear_album` (con fotos o vacío), `renombrar_album`, `eliminar_album`,
       `anadir_a_album` y `quitar_de_album` (`src-tauri/src/albumes.rs`); la
       portada lleva su versión de edición (`cover_edit_version`).
4. [x] Capturas del diseño (pantalla Álbumes, dentro de un álbum, «Añadir a
       álbum» y eliminar), en claro y oscuro, aprobadas por Paulo el 12 nov
       (`capturas/014-*.png`).
5. [x] Pantalla Álbumes y entrada del menú (`lib/albumes/`), con «Nuevo álbum»
       y estado vacío; pruebas. Al pulsar una tarjeta se abre el álbum con la
       cuadrícula (`navegacion.album`); su cabecera es la tarea 6.
6. [x] Vista de un álbum: cabecera, cambiar nombre, eliminar y cuadrícula;
       pruebas. `CabeceraAlbum.svelte`: «Álbumes» para volver, nombre editable
       en el sitio, «N fotos · rango» y menú ⋮; `Esc` vuelve a Álbumes.
7. [ ] Barra de selección: «Añadir a álbum» con su lista y «Quitar del álbum»
       con «Deshacer»; pruebas.
8. [ ] Prueba de originales intactos con álbumes y `docs/progreso.md`.
