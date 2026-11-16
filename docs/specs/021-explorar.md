# 021 · Explorar

- Estado: en curso
- Fase: F2 (fase A) · F4 (fases B y C)

## Qué y por qué
Quiero que la entrada «Explorar» del menú lateral, hoy desactivada como
«Próximamente», sirva para encontrar fotos sin recorrer la línea de tiempo:
recuerdos de estas fechas en otros años, los vídeos, las capturas de pantalla,
las panorámicas, lo último que importé… igual que la pestaña Explorar (o
«Buscar») de Google Fotos.

Google Fotos arma esa pantalla con IA en sus servidores (personas, cosas,
documentos). MNEMOS es local y privado, así que el plan va por fases: primero
todo lo que sale de los datos que ya tenemos, sin IA ni internet; después
lugares y cámaras, que necesitan guardar datos nuevos al importar; y por último,
solo si se decide, personas y cosas con modelos locales.

Explorar solo enseña colecciones calculadas: nunca mueve, copia ni cambia
archivos (regla 1 de CLAUDE.md).

## Qué tiene Google Fotos y qué haría MNEMOS

| Google Fotos (Explorar) | Cómo lo arma Google | MNEMOS | Fase |
|---|---|---|---|
| Recuerdos «Hace N años» | Fecha | Fecha de captura | A |
| Vídeos | Tipo de archivo | `media_kind` (spec 019) | A |
| Capturas de pantalla y grabaciones | IA y origen | Nombre del archivo | A |
| Panorámicas | Metadatos | Proporción ancho/alto | A |
| Añadido recientemente | Fecha de subida | `imported_at` | A |
| Favoritos | Estrella | Ya está en el menú lateral | — |
| Cosas (playa, perros, comida…) | IA | Etiquetas propias (spec 016) | A |
| Lugares y mapa | GPS + Google Maps | GPS + lista de ciudades sin conexión | B |
| Cámara (búsqueda) | EXIF | Marca y modelo del EXIF | B |
| Selfies | IA y cámara frontal | Objetivo del EXIF («front») | B |
| Personas y mascotas | Reconocimiento facial | Modelo local, opcional | C |
| Documentos, recibos, texto en fotos | IA y OCR | Modelo local, opcional | C |
| Ask Photos, búsqueda por contenido | IA en la nube | Fuera de alcance | — |

## Fases

### Fase A · Explorar con lo que ya hay (esta spec, F2)
Pantalla Explorar con recuerdos, colecciones automáticas y etiquetas. Sale
toda de columnas que ya existen: no necesita migración ni datos nuevos. Es lo
que se detalla en el resto del documento.

### Fase B · Lugares, cámaras y selfies (spec aparte, F4)
- **Guardar más datos al importar** (migración nueva): GPS del EXIF (hoy solo
  se guarda el del Takeout y el de los vídeos), marca, modelo y objetivo de la
  cámara. `mnemos index` los rellena en una biblioteca ya importada.
- **Lugares sin conexión:** la app lleva una lista de ciudades de GeoNames
  (las de más de 15 000 habitantes, unas 30 000, alrededor de 1 MB comprimida;
  licencia CC BY 4.0, compatible con GPL) y asigna a cada foto la ciudad más
  cercana. Fila «Lugares» en Explorar con una tarjeta por ciudad, búsqueda por
  lugar («Oviedo») y «Lugar» en el panel Detalles.
- **Mapa:** un mapamundi sencillo dibujado en SVG con un punto por ciudad. Un
  mapa con calles necesita teselas de internet o cientos de MB en el disco, así
  que queda fuera salvo que se decida otra cosa.
- **Cámaras:** fila «Cámaras» (una tarjeta por modelo) y búsqueda por cámara.
- **Selfies:** las de cámara frontal según el objetivo del EXIF. Solo funciona
  con móviles que lo escriben (iPhone y muchos Android).

### Fase C · Personas y cosas con IA local (por decidir, muy posterior)
- Detección y agrupación de caras, y búsqueda por contenido («playa»,
  «perro»), con modelos que se ejecutan en el equipo (ONNX). Nada sale del
  disco.
- Coste: modelos de 100 a 400 MB dentro de la carpeta portátil, varios minutos
  de proceso por cada mil fotos en un portátil y mucho trabajo de interfaz
  (nombrar personas, unir, corregir).
- Se deja escrito para que la pantalla tenga sitio para estas filas, pero no se
  planifica hasta que la fase B esté probada y Paulo decida si merece la pena.

## Comportamiento (fase A)

### Entrada y pantalla
- La entrada «Explorar» del menú lateral se activa y abre la pantalla, debajo
  de «Fotos» como ahora.
- La barra superior conserva la búsqueda. Buscar desde Explorar enseña los
  resultados como en cualquier otra sección.
- Cabecera: título «Explorar» y debajo, en gris, «Recuerdos y colecciones de tu
  biblioteca».
- Debajo, filas con título. Cada fila es una tira horizontal de tarjetas con
  portada, que se desplaza con flechas a los lados si no cabe (y con la rueda o
  el panel táctil en horizontal). Una fila sin tarjetas no se muestra.
- Las tarjetas y portadas son las de la pantalla Álbumes (spec 014): portada
  con esquinas redondeadas, con su edición si la tiene, nombre debajo y número
  de elementos en gris.

### Fila «Recuerdos»
- Una tarjeta por cada año anterior que tenga fotos en **esta semana**: desde 3
  días antes de hoy hasta 3 días después, con la fecha de captura en hora local
  (la misma que usan las cabeceras de día).
- Tarjeta: portada vertical (3:4), más alta que las demás, con un degradado
  oscuro abajo y el texto encima: «Hace 3 años» y debajo «12–18 nov 2023».
- Orden: del año más reciente al más antiguo.
- Portada: la favorita más reciente de esa semana; si no hay, la más reciente.
- Si ningún año tiene fotos en esta semana, la fila no se muestra.

### Fila «Colecciones»
Tarjetas cuadradas, en este orden, cada una solo si tiene al menos un
elemento:
- **Vídeos:** `media_kind = 'video'`. En la portada, la duración del vídeo como
  en la cuadrícula (spec 019).
- **Capturas de pantalla:** por el nombre del archivo, sin distinguir
  mayúsculas ni tildes: empieza por `Screenshot`, `Screen Shot`,
  `Screen_Recording`, `Captura de pantalla`, `Captura`, `Grabación de pantalla`,
  `Scr_` o contiene `_screenshot`. La lista vive en un solo sitio del núcleo
  para poder ampliarla.
- **Panorámicas:** fotos con el lado largo al menos 2 veces el corto, teniendo
  en cuenta la orientación EXIF. Los vídeos no cuentan.
- **Editadas:** fotos con una edición a la vista (`edit_version`, spec 009).
- **RAW:** fotos con extensión RAW o DNG (las que tienen vista previa
  incrustada, spec 002).
- **Añadidas recientemente:** todas las fotos y vídeos (sin archivadas ni
  Papelera), por fecha de importación. La tarjeta dice «Última importación:
  14 nov» en lugar del número.

### Fila «Tus etiquetas»
- Las 10 etiquetas con más fotos (spec 016), con la portada de cada una.
- Al final de la fila, una tarjeta «Ver todas» que lleva a la pantalla
  Etiquetas.
- Sin etiquetas, la fila no se muestra. Hace el papel de «Cosas» de Google
  Fotos con lo que el usuario ha etiquetado a mano.

### Abrir una tarjeta
- Abre las fotos en la cuadrícula de siempre (días, barra de meses, selección,
  barra de selección y visor) con una cabecera como la de un álbum: «Explorar»
  con flecha para volver, el nombre en grande y debajo «N elementos» o, en un
  recuerdo, «N elementos · 12–18 nov 2023».
- Sin menú ⋮ ni nombre editable: las colecciones no se pueden renombrar ni
  borrar.
- **Añadidas recientemente** agrupa y ordena por día de importación («Importadas
  el 14 nov»), como la Papelera agrupa por fecha de envío (spec 015).
- Las demás ordenan por fecha de captura, de la más reciente a la más antigua.
- `Esc` sin selección ni búsqueda vuelve a Explorar.
- Las acciones de la barra de selección funcionan como en Fotos. Si una acción
  saca la foto de la colección (archivar, papelera, quitar la edición), sale de
  la cuadrícula como en Favoritos.

### Archivadas y Papelera
- Igual que en Fotos: las colecciones y los recuerdos no incluyen las fotos
  archivadas ni las de la Papelera, y tampoco cuentan.
- Las etiquetas siguen su propia regla (spec 016: archivadas sí, Papelera no).

### Biblioteca vacía o pequeña
- Biblioteca sin fotos: el mismo estado vacío que Fotos, con «Importar fotos».
- Con fotos pero sin ninguna fila: «Aún no hay nada que explorar. Aquí saldrán
  tus recuerdos, vídeos, capturas y panorámicas en cuanto los haya.»

## Datos
- Sin migración. Todo sale de columnas que ya existen: `taken_at`,
  `imported_at`, `media_kind`, `width`, `height`, `orientation`,
  `edit_version`, `rel_path`, `favorite`, `archived_at`, `trashed_at` y las
  tablas de etiquetas.
- Núcleo:
  - `View::Collection(Collection)` con `Collection` = `videos`, `capturas`,
    `panoramicas`, `editadas`, `raw` y `recientes`, serializada como
    `{"coleccion": "videos"}`. Entra en meses, páginas, `Scope` y selección
    como las demás vistas. `recientes` ordena por `imported_at` (`Filter::order`,
    como la Papelera).
  - `View::Range { desde, hasta }` (`{"rango": {...}}`) para abrir un recuerdo:
    fotos con fecha de captura entre dos instantes, sin archivadas.
  - `explore(hoy)` devuelve en una sola llamada las tres filas: recuerdos (año,
    rango, número y portada), colecciones (tipo, número, portada y, para
    `recientes`, la fecha de la última importación) y etiquetas (las 10 con más
    fotos).
- Comando de Tauri `explorar` y envoltorio en `biblioteca.ts`; `Vista` gana las
  dos formas nuevas.
- Rendimiento: `explore` en menos de 100 ms con 10 000 fotos sin índices
  nuevos. Si no llega, se añaden índices parciales en una migración (se
  reservaría la 0016).

## Casos límite
- Fotos sin fecha de captura: no salen en Recuerdos; sí en las colecciones.
- Fotos de antes de 1970 o con fecha en el futuro: Recuerdos solo mira años
  anteriores al actual.
- Una semana que cruza el año (29 dic – 4 ene): el rango va de un año al
  siguiente y la tarjeta dice «29 dic 2022 – 4 ene 2023».
- 29 de febrero: la ventana de 7 días lo cubre en los años no bisiestos.
- Fotos desaparecidas (spec 002): cuentan y salen con «Sin vista previa», como
  en Fotos.
- Una foto en varias colecciones (un vídeo de captura de pantalla) sale en las
  dos.
- Disco desconectado: Explorar enseña el error de lectura de la biblioteca en
  lugar de las filas, igual que Álbumes.
- La pantalla se recalcula al entrar y después de importar, editar, archivar o
  enviar a la Papelera (`cambios.svelte.ts`), no en cada pintado.

## Criterios de aceptación
- [ ] La entrada «Explorar» del menú lateral abre la pantalla con las filas
      Recuerdos, Colecciones y Tus etiquetas; las filas vacías no salen.
- [ ] Recuerdos enseña una tarjeta por año con fotos en la semana de hoy, con
      prueba de los casos de fin de año y 29 de febrero.
- [ ] Vídeos, Capturas de pantalla, Panorámicas, Editadas, RAW y Añadidas
      recientemente cuentan y abren las fotos correctas (pruebas en el núcleo).
- [ ] Abrir una tarjeta enseña la cuadrícula con selección y visor, y `Esc`
      vuelve a Explorar.
- [ ] Archivadas y Papelera quedan fuera de las colecciones y los recuerdos.
- [ ] `explore` tarda menos de 100 ms con 10 000 fotos (`docs/rendimiento.md`).
- [ ] Los originales no se modifican (prueba automática en
      `crates/mnemos-core/tests/originales_intactos.rs`).

## Fuera de alcance (fase A)
- Lugares, mapa, cámaras y selfies (fase B).
- Personas, cosas, documentos y texto dentro de las fotos (fase C).
- Recuerdos en la pantalla Fotos (fila de tarjetas encima de la cuadrícula),
  presentaciones, collages y vídeos automáticos.
- Crear un álbum a partir de una colección en un clic (se puede con
  «Seleccionar todo» y «Añadir a álbum»).
- Colecciones definidas por el usuario (búsquedas guardadas).
- Ocultar o reordenar las filas de Explorar.

## Tareas (≤1 h cada una)
1. [x] Capturas del diseño (Explorar, colección abierta, recuerdo abierto y
       estado vacío), en claro y oscuro, aprobadas por Paulo el 16 nov
       (`capturas/021-*.png`).
2. [x] Núcleo: `Collection` y `View::Collection` (videos, capturas,
       panorámicas, editadas, raw) en meses, páginas y `Scope`; lista de
       patrones de capturas en un solo sitio; pruebas. Hecho en `crates/mnemos-core/src/collections.rs`
       (`SCREENSHOT_PATTERNS` mira solo el nombre del archivo, no la carpeta).
3. [x] Núcleo: `recientes` ordenada y agrupada por `imported_at`, y
       `View::Range` para los recuerdos; pruebas. `Collection::Recent`
       (`{"coleccion": "recientes"}`, `GridPhoto::date` es la fecha de
       importación) y `View::Range { desde, hasta }` (`{"rango": {...}}`, solo
       fotos con `taken_at` en `[desde, hasta)`).
4. [ ] Núcleo: `explore(hoy)` con las tres filas y sus portadas; pruebas de fin
       de año, 29 de febrero, archivadas y Papelera; medida con 10 000 fotos en
       `docs/rendimiento.md`.
5. [ ] Comando de Tauri `explorar`, envoltorios en `biblioteca.ts` y las dos
       formas nuevas de `Vista`.
6. [ ] Pantalla Explorar (`lib/explorar/`): entrada del menú, filas con
       desplazamiento horizontal y flechas, tarjetas, estados vacíos;
       `Seccion` gana `explorar` en `navegacion.svelte.ts`.
7. [ ] Abrir una colección o un recuerdo: cabecera con «Explorar» para volver,
       cuadrícula con selección y visor, `Esc`, cabeceras «Importadas el …» en
       Añadidas recientemente, recarga tras cambios.
8. [ ] `tests/originales_intactos.rs` y sección «Explorar» en
       `docs/arquitectura.md`.
9. [ ] Probarlo en la app real con la biblioteca del disco (Paulo).
