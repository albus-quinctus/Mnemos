# 004 · Fecha de captura

- Estado: hecha
- Fase: F1

## Qué y por qué
Quiero ver mis fotos ordenadas por el día en que las hice, no por el día en que
las copié. Al extraer un zip de Google Takeout, todas las fotos tienen la fecha
de modificación de la exportación y acaban juntas en el mismo mes.

## Comportamiento
- La fecha de captura (`photos.taken_at`) sale de la primera fuente que la tenga:
  1. EXIF `DateTimeOriginal` (o `DateTimeDigitized`), con `OffsetTimeOriginal`
     si la cámara lo guarda.
  2. El nombre del archivo, si trae fecha y hora: `IMG_20190512_143000`,
     `IMG20220110212038`, `PXL_20190512_143000123`, `20190512_143000`,
     `Screenshot_2019-05-12-14-30-00`, `WhatsApp Image 2019-05-12 at 14.30.00`,
     `Captura de pantalla 2019-05-12 a las 14.30.00`, `photo_57@03-01-2023_21-18-03`
     y marcas Unix en milisegundos (`1586252852190_15192911`).
  3. Solo al importar: `photoTakenTime` del `.json` que Google Takeout deja al
     lado de la foto (`foto.jpg.supplemental-metadata.json`, `foto.jpg.json`,
     nombres truncados, copias `foto(1).jpg` (y `foto (1).jpg` con su propio `.json`)
     y versiones `-editada`). Va detrás
     del nombre con hora porque, en fotos sin EXIF, Google a veces guarda otra
     fecha (en el Takeout de prueba, 11 horas antes que la del nombre).
  4. El nombre del archivo con solo el día: `IMG-20181215-WA0003`.
- Si no hay ninguna, la cuadrícula sigue usando la fecha de modificación.
- Se guarda de dónde salió (`photos.taken_source`: `exif`, `nombre`, `takeout`,
  `nombre_dia` o `ninguna`) para no volver a buscarla y para mejorarla después.
- Una fecha sin zona horaria (EXIF sin desfase, nombre del archivo) es hora
  local: se muestra con la misma hora que marcaba la cámara.
- El panel Detalles del visor muestra la fecha de captura y de dónde sale.
- Al importar, la carpeta `Originales/AAAA/MM/` usa la fecha de captura.
- `mnemos index` rellena la fecha de las fotos que aún no la tienen, con EXIF y
  nombre. Volver a importar la misma carpeta de Takeout completa las fechas de
  las fotos ya importadas con sus `.json`: las fotos repetidas no se copian,
  solo se mejora su fecha.

## Casos límite
- EXIF vacío (`0000:00:00 00:00:00`), fechas imposibles o anteriores a 1900: se ignoran.
- Números en el nombre que parecen fechas pero no lo son (`DSC01234`, `12345678`): solo
  cuentan años entre 1990 y 2100 y meses, días y horas válidos.
- `.json` dañado, sin `photoTakenTime` o de otra foto: se ignora.
- Los originales ya importados no se mueven de carpeta (regla 1); solo cambia su orden.

## Criterios de aceptación
- [x] Una foto con EXIF se ordena por su `DateTimeOriginal` aunque su mtime sea otro.
- [x] Una foto de Takeout sin EXIF toma la fecha de su `.json`, también con nombres truncados y `(1)`.
- [x] Una foto sin EXIF ni `.json` toma la fecha de su nombre.
- [x] El orden de prioridad es EXIF, nombre con hora, `.json` y nombre con solo el día.
- [x] `index` rellena las fotos ya registradas sin fecha.
- [x] Reimportar una carpeta de Takeout mejora la fecha de las repetidas sin copiarlas.
- [x] Los originales no se modifican (prueba automática)

## Fuera de alcance
- Editar la fecha a mano (F2).
- Leer GPS, descripción o álbumes del `.json` de Takeout.
- Mover los originales ya importados a la carpeta de su fecha.

## Tareas (≤1 h cada una)
1. [x] Leer la fecha EXIF en `mnemos-imaging`.
2. [x] Fecha en el nombre del archivo y en el `.json` de Takeout (`mnemos-core`).
3. [x] Migración 0006 (`taken_source`) e indexado.
4. [x] Importación: carpeta por fecha de captura y mejora de las repetidas.
