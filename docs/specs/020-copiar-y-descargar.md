# 020 · Copiar, descargar y «Guardar como»

- Estado: en curso
- Fase: F2

## Qué y por qué
Quiero hacer con una foto lo que hago en Google Fotos o en el navegador:
copiarla para pegarla en un chat o un documento, bajarla a mi carpeta de
Descargas sin preguntas, o guardarla con el nombre y en el sitio que yo
elija. Hoy MNEMOS solo tiene «Exportar» (specs 008 y 017), que siempre pide
una carpeta, y el clic derecho sobre la foto saca el menú del motor web, cuyo
«Copiar imagen» falla.

Reutiliza la exportación de la spec 008 (misma regla de qué se saca de una
foto editada, mismos nombres libres, nunca dentro de la biblioteca) y el lote
de la spec 017. Lo único nuevo de verdad es el portapapeles.

## Comportamiento

### Dónde están las acciones
- **Menú del visor:** el botón de la flecha que sale de la bandeja pasa a
  llamarse «Copiar, descargar o exportar». Su menú queda así
  (`capturas/020-menu-*.png`):
  - **Copiar imagen** · `Ctrl` + `C` · «Para pegarla en otra aplicación».
  - **Descargar** · `Mayús` + `D` · «A Descargas, el archivo tal cual» (o
    «A Descargas, con las ediciones» si se ve una versión editada).
  - **Guardar como…** · `Ctrl` + `Mayús` + `S` · «Eliges carpeta y nombre».
  - Una raya y el título «Exportar a una carpeta», con las dos opciones de la
    spec 008 sin cambios (tamaño original y reducido).
- **Clic derecho sobre la foto** en el visor: un menú propio con Copiar
  imagen, Descargar y Guardar como…, con sus atajos
  (`capturas/020-contextual-*.png`). Sustituye al menú del motor web. En la
  Papelera no sale (allí solo se restaura o se elimina).
- **Barra de selección:** el menú «Exportar» gana arriba «Descargar · A
  Descargas, en tamaño original», separado por una raya de las dos opciones de
  la spec 017 (`capturas/020-barra-*.png`).
- **Teclado en la cuadrícula:** con una sola foto seleccionada, `Ctrl` + `C`
  la copia; con varias, aviso «Solo se puede copiar una foto a la vez» (como
  Google Fotos, que tampoco copia varias). `Mayús` + `D` descarga la
  selección.

### Copiar imagen
- Pone en el portapapeles del sistema **la foto tal como se ve**: girada según
  su orientación y con la edición aplicada si hay una versión a la vista
  (spec 009). Se pega como imagen en cualquier aplicación (chat, documento,
  editor de imágenes).
- **Tamaño:** completo, sin reducir (Paulo lo eligió el 15 nov). Una foto de
  24 MP son unos 100 MB en memoria y puede tardar unos segundos en copiarse y
  pegarse; mientras, el aviso «Copiando…».
- Va **sin metadatos**: el portapapeles de imágenes no los lleva.
- Mientras se prepara, el aviso «Copiando…»; al terminar, «Imagen copiada»
  unos segundos (`capturas/020-copiada-*.png`).
- **RAW:** se copia su vista previa incrustada, como la ve el visor.
- **No disponible** (opción desactivada con el motivo debajo):
  - Vídeos: «Los vídeos no se pueden copiar».
  - Formatos que aún no se decodifican (HEIC, AVIF, RAW sin vista previa):
    al pulsar, «Esta foto no se puede copiar todavía. Usa Descargar».
- `Ctrl` + `C` no actúa si el foco está en un campo de texto (fecha manual,
  etiquetas) o si hay texto seleccionado: ahí copia el texto, como siempre.

### Descargar
- Guarda la foto **sin preguntar** en la carpeta de Descargas del sistema
  (`~/Descargas` o `~/Downloads` según el idioma, la de Windows y la de macOS).
  Es lo mismo que «Exportar · Tamaño original» con destino fijo: el archivo tal
  cual, verificado con BLAKE3; o, si se ve una versión editada, la foto
  editada a tamaño completo con el EXIF del original (spec 008).
- **Nombres:** nunca se sobrescribe; `IMG_2003 (1).jpg` si ya existe.
- **Aviso:** «Guardada en Descargas · IMG_2003.jpg» con el enlace «Mostrar en
  la carpeta», que abre el gestor de archivos en esa carpeta (con el archivo
  marcado donde el sistema lo permite) (`capturas/020-descargada-*.png`).
- **Varias desde la barra:** el lote de la spec 017 con destino Descargas, en
  tamaño original, con su misma tarjeta de progreso, «Cancelar» y resumen. Al
  terminar, la tarjeta añade «Mostrar en la carpeta».
- Vídeos incluidos: se descargan tal cual.

### Guardar como…
- Abre el diálogo **«Guardar»** del sistema (archivo, no carpeta) con el
  nombre de la foto ya puesto y la última carpeta usada en la sesión (la misma
  que Exportar). Si se cancela, no pasa nada.
- Guarda lo mismo que Descargar (tamaño original, con la edición si se ve).
- **Nombre:** se respeta el que escribe el usuario. Si no acaba en la
  extensión del formato que se guarda, se le añade (`viaje` → `viaje.jpg`;
  una editada de un HEIC sale `.jpg`). No se convierte de formato por cambiar
  la extensión.
- **Sobrescribir:** aquí sí, porque el diálogo del sistema ya pregunta
  «¿Reemplazar?». Se escribe a un temporal en la misma carpeta y se renombra
  encima al terminar, así que si falla, el archivo anterior queda intacto.
- Aviso «Guardada en <ruta completa>» con «Mostrar en la carpeta».
- Solo desde el visor (una foto). Para varias, Exportar.

### Siempre
- Ninguna de las tres cambia nada en la biblioteca: ni la base de datos ni
  `Originales/`. Sin migración.
- `Esc` con un menú abierto cierra el menú, no el visor.

## Cómo se hace (técnico)
- **Portapapeles:** `tauri-plugin-clipboard-manager` (2.x), llamado **desde
  Rust**, no desde la interfaz: la imagen no cruza al motor web y no hace
  falta abrir su permiso en `capabilities/`. Por dentro usa `arboard`, ya con
  soporte de Wayland (`wayland-data-control`): funciona en Sway, KDE y
  Hyprland; en X11 y XWayland también. En Windows pone la imagen como DIB y
  PNG; en macOS como imagen del sistema.
- `mnemos-imaging`: `clipboard_image(path, orientation, geometry) -> RgbaImage`,
  con la misma decodificación que el visor y la exportación reducida (RAW por
  su vista previa, receta aplicada).
- Comando `copiar_imagen(id)` fuera del bloqueo de la biblioteca, en un hilo
  aparte (decodificar tarda).
- Comando `descargar(id)` = `export(root, media, download_dir, Original)`;
  `descargar_varias(ids)` = `exportar_varias` con la carpeta de Descargas
  (`app.path().download_dir()`).
- Comando `guardar_como(id, ruta)`: el diálogo lo abre la interfaz con
  `plugin-dialog` (`save`, añadir `dialog:allow-save`); el núcleo gana
  `export_to_file(root, media, dest_file)` que escribe a temporal y renombra.
- «Mostrar en la carpeta»: comando `mostrar_en_carpeta(ruta)` como
  `abrir_con_otra_aplicacion` de la spec 019 (`xdg-open` de la carpeta en
  Linux, `explorer /select,` en Windows, `open -R` en macOS).

## Casos límite
- **Linux, el portapapeles vive con la app:** en Wayland y X11 la imagen la
  sirve MNEMOS mientras está abierta. Si se cierra antes de pegar, se pierde,
  salvo que haya un gestor de portapapeles (cliphist, CopyQ, el de KDE). Pasa
  igual con cualquier app; se documenta en el LEEME, no en la interfaz.
- **GNOME en Wayland** no tiene el protocolo `data-control`: `arboard` usa
  XWayland, y pegar en apps nativas de Wayland puede fallar. Se prueba; si
  falla, error claro «No se pudo copiar al portapapeles».
- **Sin carpeta de Descargas** (sistema raro o ruta que no existe): «No se
  encuentra la carpeta Descargas. Usa Guardar como…».
- **Descargas dentro de la biblioteca** (biblioteca en `~/Descargas/MNEMOS`):
  se rechaza como en la spec 008, con «Elige otra carpeta con Guardar como…».
- **Guardar como encima del propio original** o en cualquier sitio dentro de la
  biblioteca: rechazado («Elige una carpeta fuera de la biblioteca»).
- **Original que falta en el disco o disco desconectado:** «El archivo no está
  en el disco», como en la spec 008.
- **Pulsar Copiar dos veces seguidas** o cambiar de foto mientras se copia:
  gana la última; el aviso dice el resultado de la que quedó.
- **Disco lleno o sin permiso** al descargar o guardar: el error de E/S en
  lenguaje claro; nada a medias (temporal borrado).
- Rutas con espacios, tildes o caracteres especiales en Windows, macOS y Linux.

## Criterios de aceptación
- [x] `clipboard_image` devuelve la foto girada, con la receta aplicada y a tamaño completo; un RAW usa su vista previa; HEIC da error claro (prueba automática).
- [ ] Copiar imagen en Sway pega la foto en otra app (Paulo, a mano), también en Windows y macOS si hay equipo.
- [ ] Descargar deja en la carpeta de Descargas lo mismo que «Exportar · Tamaño original», con `nombre (N).ext` si se repite (prueba automática con una carpeta temporal como Descargas).
- [ ] Guardar como escribe el nombre elegido, añade la extensión que falta, reemplaza el existente solo al terminar y rechaza destinos dentro de la biblioteca (prueba automática).
- [ ] Descargar varias usa el lote de la spec 017 y su tarjeta (prueba de la interfaz).
- [ ] Menú del visor, clic derecho, barra y atajos (`Ctrl` + `C`, `Mayús` + `D`, `Ctrl` + `Mayús` + `S`) funcionan con ratón y teclado, y `Ctrl` + `C` no roba la copia de texto en campos (prueba de la interfaz).
- [ ] Capturas del diseño aprobadas por Paulo.
- [ ] Los originales y la base de datos no se modifican (prueba automática).

## Fuera de alcance
- Copiar varias fotos a la vez, o copiar el **archivo** (para pegarlo en el
  gestor de archivos como `text/uri-list`). Podría venir después: así sí se
  pegarían vídeos y varias fotos.
- Descargar varias en un `.zip` (Google Fotos lo hace; aquí la carpeta ya
  sirve).
- Elegir otra carpeta fija de descargas en Ajustes.
- Arrastrar una foto desde MNEMOS a otra app.
- Compartir con otras apps o por correo (regla 4: sin nube).
- Clic derecho en la cuadrícula (la barra de selección ya lo cubre).

## Tareas (≤1 h cada una)
1. [x] Spec y capturas del diseño (`capturas/020-*.png` en los archivos del proyecto). Spec fusionada el 15 nov.
2. [x] `mnemos-imaging`: `clipboard_image` a tamaño completo, reutilizando la decodificación del visor. Pruebas. Devuelve un `RgbaImage`.
3. [x] `mnemos-core`: `export_to_file` (temporal y renombrado, extensión añadida, nunca dentro de la biblioteca). Pruebas. Con `Library::save_photo_as`; sus dos llamadas nuevas están en la lista de `tests/solo_la_papelera_borra.rs`.
4. [x] App: `tauri-plugin-clipboard-manager` y comando `copiar_imagen` en hilo aparte. `cargo deny` en verde. En `apps/desktop/src-tauri/src/copiar.rs`; `deny.toml` admite la licencia `BSL-1.0` (de `clipboard-win`, el portapapeles de Windows).
5. [x] App: comandos `descargar`, `descargar_varias`, `guardar_como` y `mostrar_en_carpeta`; permiso `dialog:allow-save`. Envoltorios en `biblioteca.ts`.
6. [x] Visor: menú nuevo, clic derecho propio, atajos y aviso con «Mostrar en la carpeta». Pruebas de la interfaz. Atajos y rutas en `lib/copiar/copiar.ts`, con pruebas.
7. [x] Cuadrícula y barra: «Descargar» en el menú, `Ctrl` + `C` y `Mayús` + `D` con la selección. Pruebas de la interfaz. `descargarSeleccion` en `lib/exportacion/`; la tarjeta dice «Descargando…» y, al terminar, «Mostrar en la carpeta» (también al exportar).
8. [ ] AppImage nueva y prueba real de Paulo en Sway: copiar y pegar en otra app, Descargar, Guardar como.
