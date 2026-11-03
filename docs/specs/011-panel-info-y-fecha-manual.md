# 011 · Panel de información y fecha a mano

- Estado: en curso
- Fase: F2

## Qué y por qué
Quiero ver con qué cámara y con qué ajustes hice una foto sin salir del visor,
y poder corregir su fecha cuando está mal (un escaneo, una foto de WhatsApp sin
EXIF, una cámara con el reloj atrasado) para que aparezca en su sitio de la
línea de tiempo.

## Comportamiento
- El panel Detalles del visor (tecla `i`, spec 002) añade una sección
  **Cámara** con lo que traiga el EXIF: marca y modelo, objetivo, exposición
  (`1/250 s`), apertura (`ƒ/2,8`), ISO, distancia focal (con su equivalente en
  35 mm si lo trae), flash y coordenadas GPS en texto (`41,38513° N, 2,17340° E`).
  Solo se muestran las filas que existen; sin EXIF, la sección no aparece.
- El EXIF se lee del original al abrir el panel, no se guarda en la base de
  datos. Si el disco no está, el panel muestra el resto de los detalles.
- Junto a la fecha de captura hay un botón **Cambiar fecha**. Abre un pequeño
  formulario con día y hora (segundos opcionales), ya relleno con la fecha
  actual de la foto (o la de modificación si no tiene). **Guardar** la aplica y
  **Cancelar** (o `Esc`) lo cierra sin cambios.
- La fecha a mano es hora local, como la del EXIF sin zona horaria (spec 004):
  se ve con la misma hora que se escribió.
- Se guarda con `taken_source = 'manual'` y el panel la marca como
  «Puesta a mano». La fecha automática que tenía se conserva aparte y el botón
  **Volver a la fecha automática** la restaura.
- Ningún proceso automático pisa una fecha a mano: ni `mnemos index` (aunque el
  archivo cambie), ni reimportar un Takeout. Si encuentran una fecha automática
  mejor, la guardan como la automática de reserva.
- Al cerrar el visor, las cuadrículas abiertas (fotos y resultados de búsqueda)
  se recargan sin perder la posición, y la foto aparece en su nuevo día. Mientras
  el visor sigue abierto la foto no se mueve de la tira, para no perderla.
- Solo cambia la base de datos: el archivo original no se toca (regla 1).

## Casos límite
- Fechas imposibles (`31/02`), anteriores a 1900 o posteriores a 2200: se
  rechazan con un aviso y no se guarda nada.
- Foto que ya no existe en la base de datos (borrada por otro proceso): el
  guardado no hace nada y el panel avisa.
- Volver a poner a mano una fecha ya manual no pierde la automática guardada.
- Fotos sin fecha automática (`ninguna` o sin buscar): al volver a la
  automática se queda igual que antes, y `index` la sigue buscando si no se
  había buscado.
- RAW con contenedor propio (CR3, RAF, ORF, RW2) cuyo EXIF no se puede leer:
  no hay sección Cámara.
- Cambio de hora de verano: la hora escrita se pasa a UTC con la zona del
  equipo, igual que el EXIF sin desfase.

## Criterios de aceptación
- [x] El panel muestra cámara, objetivo, exposición, apertura, ISO, focal, flash
      y GPS de una foto con EXIF, y nada de eso en una sin EXIF.
- [x] Guardar una fecha a mano cambia `taken_at` a esa hora local y
      `taken_source` a `manual`.
- [x] Volver a la fecha automática deja `taken_at` y `taken_source` como estaban.
- [x] `index` con el archivo cambiado y reimportar un Takeout no pisan la fecha a mano.
- [x] Una fecha imposible se rechaza sin cambiar nada.
- [ ] La cuadrícula refleja el nuevo orden al cerrar el visor (falta probarlo dentro de Tauri).
- [x] Los originales no se modifican (prueba automática).

## Fuera de alcance
- Cambiar la fecha de varias fotos a la vez o desplazarlas un número de horas
  (útil para una cámara con el reloj mal; irá con la selección múltiple de la
  spec 006).
- Escribir la fecha en el EXIF del archivo (rompería la regla 1).
- Mapa de la ubicación (exigiría conexión) y editar el GPS.
- Guardar el EXIF en la base de datos para buscar por cámara u objetivo.
- Descripción o pie de foto.

## Diseño
- Migración 0011 (`0011_fecha_manual.sql`): `photos.taken_auto_at` y
  `photos.taken_auto_source`, la fecha automática de reserva. Solo tienen valor
  cuando `taken_source = 'manual'`. Las migraciones se aplican por su posición
  en `db.rs`: si 0008–0010 se fusionan antes, esta va detrás de ellas.
- `mnemos-imaging::camera_info(path)` lee el EXIF con `kamadak-exif`.
- `mnemos-core`: `Library::set_manual_taken(id, "AAAA-MM-DD hh:mm:ss")` y
  `Library::clear_manual_taken(id)`. `manual` tiene el rango más alto en
  `Source::RANK_SQL`, así que `improve_taken` nunca la reemplaza.
- Tauri: comandos `camara`, `poner_fecha` y `quitar_fecha_manual`.

## Tareas (≤1 h cada una)
1. [x] Migración 0011 y fecha a mano en el núcleo, con pruebas.
2. [x] Lectura de los datos de cámara en `mnemos-imaging`, con pruebas.
3. [x] Comandos de Tauri.
4. [x] Sección Cámara y formulario de fecha en el panel del visor.
5. [x] Recargar las cuadrículas al cerrar el visor si cambió alguna fecha.
