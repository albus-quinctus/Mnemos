# 024 · Filtros de búsqueda y estrellas

- Estado: en curso
- Fase: F2 (0.5 beta)

## Qué y por qué
Quiero poder dar de 1 a 5 estrellas a mis fotos y vídeos, y encontrar fotos
combinando el texto de la búsqueda con filtros: solo vídeos, solo las de una
cámara, las de un verano, las de 4 estrellas o más, las favoritas de un álbum…
Si ya valoré o etiqueté fotos en otro programa (Lightroom, darktable, digiKam,
el explorador de Windows), quiero que MNEMOS lo lea al importarlas.

MNEMOS lee esos datos de los archivos, pero **nunca escribe** en ellos ni crea
archivos `.xmp` junto a los originales (regla 1 de CLAUDE.md). Las estrellas que
se pongan en MNEMOS viven solo en la base de datos, como los favoritos y las
etiquetas.

## Comportamiento

### Qué se lee de los archivos
- **Valoración** (0 a 5): `xmp:Rating` del XMP; si no hay XMP, la etiqueta
  `Rating` del EXIF (0x4746, la que escribe Windows). Un `-1` (rechazada en
  Lightroom) cuenta como 0. Un valor fuera de rango o que no es un número se
  ignora.
- **Descripción**: `dc:description` del XMP (la entrada `x-default` o, si no
  la hay, la primera). Va a la columna `description` que ya existe (spec 007)
  **solo si está vacía**: la descripción del `.json` de Takeout gana.
- **Palabras clave**: `dc:subject` del XMP. Cada una se convierte en una
  etiqueta de MNEMOS (spec 016): se crea si no existe (sin distinguir
  mayúsculas, como hoy) y se añade a la foto. Se recortan espacios, se
  descartan las vacías y las de más de 100 caracteres. Las jerárquicas
  (`lr:hierarchicalSubject`) no se leen. Decidido por Paulo el 23 nov.
- **Cámara y objetivo** del EXIF (marca y modelo juntos, y objetivo), con la
  misma función que ya usa el panel Detalles (`camera_info`, spec 011). De un
  vídeo, la marca y el modelo de su contenedor (spec 019).

### De dónde se lee
- **XMP incrustado** en el archivo: segmento APP1 de JPEG, fragmento `iTXt`
  `XML:com.adobe.xmp` de PNG, fragmento `XMP ` de WebP y etiqueta 700 de TIFF,
  DNG y RAW. El XMP extendido de JPEG (varios segmentos) no se lee.
- **`.xmp` junto al archivo de origen, al importar**: `foto.CR2.xmp` (darktable,
  digiKam) y, si no existe, `foto.xmp` (Lightroom, Bridge). Si el `.xmp` y el
  archivo traen valores distintos, gana el `.xmp`, como en esos programas. El
  `.xmp` no se copia a la biblioteca: se lee una vez al importar y lo que trae
  queda en la base de datos.
- Los vídeos solo aportan cámara (su XMP no se lee en esta spec).

### Cuándo se lee
- Al importar, para cada foto nueva.
- Al reindexar (`mnemos index` y al abrir una biblioteca antigua), para las
  fotos que todavía no se han leído: así una biblioteca ya importada recibe la
  valoración, las palabras clave y la cámara de su XMP incrustado y su EXIF.
  Al reindexar no hay carpeta de origen, así que no hay `.xmp` al lado.
- **Una sola vez por foto** (columna `metadata_at`, como `takeout_at` en la spec
  007): si después el usuario quita una etiqueta o cambia las estrellas, ni
  reindexar ni volver a importar la misma foto lo deshace. Los originales de la
  biblioteca no cambian nunca, así que no hay nada nuevo que releer.
- Si el usuario ya puso estrellas a mano antes de que se lea el archivo (por
  ejemplo, entre la migración y el primer reindexado), las suyas ganan.

### Estrellas en el visor
- En la barra superior del visor, un botón de estrella con el número de
  estrellas («☆» si no tiene, «★ 4» si tiene). Al pulsarlo se despliega una
  fila de 5 estrellas: pasar el ratón ilumina hasta esa, pulsar la fija, pulsar
  la que ya está fijada la quita (0).
- En el panel Detalles (spec 011), una fila «Valoración» con las 5 estrellas,
  que se pueden pulsar igual.
- Teclas `1` a `5` ponen esas estrellas y `0` las quita, también en el teclado
  numérico. No hacen nada con `Ctrl`, `Alt` o `Mayús`, ni con el foco en un
  campo de texto, ni en la Papelera.
- Al cambiarla sale el aviso de siempre, abajo: «3 estrellas» o «Sin
  estrellas», con «Deshacer».

### Estrellas en la cuadrícula y la barra de selección
- Con fotos seleccionadas, la barra de selección gana el botón «Valorar» (icono
  de estrella), que abre un menú con «★★★★★» a «★» y «Quitar estrellas».
- Las teclas `0` a `5` hacen lo mismo con la selección.
- Aviso: «12 fotos con 3 estrellas» o «Estrellas quitadas de 12 fotos», con
  «Deshacer», que devuelve a cada foto el valor que tenía.
- Las miniaturas enseñan las estrellas solo si la foto tiene alguna: «★ 4»
  pequeño en la esquina inferior izquierda, sobre el mismo degradado que la
  duración de los vídeos (decidido por Paulo el 23 nov; la posición exacta se
  ve en las capturas del diseño).
- «Seleccionar todo» seguido de «Valorar» usa el mismo `Scope` que el resto de
  acciones por lotes (spec 006).

### Panel de filtros
- A la derecha del campo de búsqueda, un botón «Filtros» (icono de
  deslizadores). Lo abre y lo cierra; con filtros activos muestra cuántos hay
  («Filtros · 2»).
- El panel se despliega bajo la barra superior, sobre la cuadrícula, con:
  - **Tipo**: Todo, Fotos, Vídeos, RAW (botones de una sola opción).
  - **Cámara**: lista con las cámaras de la biblioteca y su número de fotos,
    de la que más tiene a la que menos; «Cualquiera» por defecto.
  - **Objetivo**: igual, con los objetivos. Si se ha elegido cámara, solo los
    objetivos usados con esa cámara.
  - **Fechas**: «Desde» y «Hasta» (días, ambos incluidos; se puede dejar uno
    vacío).
  - **Valoración mínima**: «Cualquiera», «★ o más» … «★★★★★».
  - **Solo favoritas**: interruptor.
  - **Álbum** y **Etiqueta**: listas con los álbumes y las etiquetas de la
    biblioteca, con buscador si hay más de 10.
  - Abajo, «Borrar filtros».
- Los filtros se aplican al momento (sin botón «Aplicar»), con la misma espera
  de 200 ms que el texto.
- Cada filtro activo sale también como una pastilla bajo la barra superior
  («Vídeos ×», «Canon EOS 80D ×», «★★★ o más ×», «Jun 2023 – Ago 2023 ×»), para
  verlos y quitarlos con el panel cerrado.
- **Se combinan con el texto**: deben cumplirse todos. Con filtros y sin texto
  también hay resultados (por ejemplo, todas las de 5 estrellas). Si el texto
  trae una fecha («marzo 2024») y además hay rango de fechas, deben cumplirse
  las dos.
- Los resultados salen como los de la búsqueda de hoy: cuadrícula por días con
  barra de meses, selección y visor. Igual que la búsqueda, incluyen las
  archivadas y no la Papelera (spec 015).
- Sin resultados: «Ninguna foto cumple estos filtros» y el botón «Borrar
  filtros».
- `Esc` cierra el panel; con el panel cerrado, `Esc` borra el texto y los
  filtros como hoy borra la búsqueda.
- Los filtros no se guardan al cerrar la app.

### Fechas: hora local
El rango de fechas usa el mismo día que las cabeceras de la cuadrícula. Cuando
se fusione la tarea de hora local de la spec 022, ese día es el de
`taken_local` (con su reserva), no el de `taken_at` pasado a la zona del
equipo. Por eso la tarea de filtros del núcleo espera a esa tarea (ver
«Coordinación»).

## Datos

### Migración 0019
```sql
-- Valoración de 0 (sin estrellas) a 5.
ALTER TABLE photos ADD COLUMN rating INTEGER NOT NULL DEFAULT 0
    CHECK (rating BETWEEN 0 AND 5);
-- De dónde salió: 'archivo' (XMP o EXIF), 'manual' (puesta en MNEMOS) o NULL.
-- La manual no se pisa nunca.
ALTER TABLE photos ADD COLUMN rating_source TEXT
    CHECK (rating_source IN ('archivo', 'manual'));
-- Cámara (marca y modelo) y objetivo del EXIF, para los filtros.
ALTER TABLE photos ADD COLUMN camera TEXT;
ALTER TABLE photos ADD COLUMN lens TEXT;
-- Cuándo se leyeron XMP, valoración EXIF y cámara (segundos Unix, UTC).
-- NULL: todavía no. Se leen una sola vez.
ALTER TABLE photos ADD COLUMN metadata_at INTEGER;

CREATE INDEX photos_rating ON photos (rating) WHERE rating > 0;
CREATE INDEX photos_camera ON photos (camera, lens) WHERE camera IS NOT NULL;
```
Se añade al final de la lista de `crates/mnemos-core/src/db.rs` en el orden en
que se fusione. Si las medidas con 10 000 fotos piden otros índices (por
ejemplo, para `media_kind` o `favorite`), van en esta misma migración antes de
fusionarla.

### Núcleo
- `mnemos-imaging`: módulo `xmp.rs` con `read_xmp(path) -> Option<XmpData>`
  (incrustado) y `read_xmp_sidecar(path) -> Option<XmpData>` (archivo `.xmp`),
  con `XmpData { rating, description, keywords }`. Usa `quick-xml` (MIT, pasa
  `cargo deny`), no resuelve entidades ni DTD, y no lee más de 4 MB de XMP. La
  valoración EXIF se lee con `kamadak-exif`, que ya usamos.
- `mnemos-core`:
  - El indexado y la importación rellenan `rating`, `rating_source`, `camera`,
    `lens`, `metadata_at`, la descripción vacía y las etiquetas, en la misma
    transacción que el resto de la foto.
  - `set_rating(ids, n) -> Vec<PreviousRating>` y `restore_ratings(..)` para
    deshacer; `set_rating` pone `rating_source = 'manual'`.
  - `SearchFilters { kind, camera, lens, from, to, min_rating, favorites,
    album, tag }` (serializado en español para la interfaz). `search_months` y
    `search_page` reciben consulta y filtros; cada filtro es una condición más
    en el `Filter` de `search_filter`, con parámetros, nunca texto pegado en el
    SQL.
  - `filter_options() -> FilterOptions` con cámaras y objetivos (con su
    número de fotos), álbumes y etiquetas, para llenar el panel. Sin Papelera.
  - El tipo RAW usa la misma lista de extensiones que la colección RAW de
    Explorar (spec 021).
- Tauri: los comandos `buscar_meses` y `buscar_fotos` ganan el argumento
  `filtros`; comandos nuevos `valorar`, `deshacer_valoracion` y
  `opciones_filtros`. `GridPhoto` y la información del visor ganan `rating`.
- Rendimiento: `search_months` y la primera página con cualquier combinación
  de filtros, en menos de 100 ms con 10 000 fotos; `filter_options` también.
  Medidas en `docs/rendimiento.md`.

## Casos límite
- XMP mal formado, cortado, enorme o con entidades: se ignora entero y la foto
  se importa igual, sin error para el usuario.
- `.xmp` que no corresponde a ninguna foto, o que es de otro programa con otro
  esquema: se ignora.
- `foto.xmp` junto a `foto.CR2` y `foto.JPG` (RAW+JPEG de Lightroom): se aplica
  a las dos, salvo a la que tenga su propio `foto.JPG.xmp`.
- Una foto repetida en la importación (misma huella, spec 001): se lee el
  primer `.xmp` que aparezca; las copias siguientes no cambian nada.
- Palabras clave que coinciden con una etiqueta existente con otras mayúsculas
  («playa» y «Playa»): se usa la existente.
- Palabras clave de Takeout y XMP a la vez: se suman.
- Descripción en Takeout y en XMP: gana la de Takeout.
- Disco desconectado a mitad de valorar un lote: la transacción no se aplica y
  sale el error de lectura de siempre; nada a medias.
- Archivos corruptos o formatos no compatibles: sin XMP ni EXIF, se quedan con
  0 estrellas y sin cámara.
- Rutas con caracteres especiales en Windows, macOS y Linux: el `.xmp` se busca
  con la misma ruta del archivo más la extensión, sin pasar por texto.
- Fotos desaparecidas (spec 002): se pueden valorar y filtrar.
- Valorar en un álbum, una etiqueta, Favoritos, Archivo o Explorar: igual que
  en Fotos. En la Papelera no hay estrellas ni teclas.
- Teclas `0` a `5` con el editor abierto (spec 009) o un diálogo: no hacen
  nada.

## Criterios de aceptación
- [ ] Importar un JPEG con `xmp:Rating="4"` y `dc:subject` lo deja con 4
      estrellas y sus palabras clave como etiquetas (pruebas con archivos de
      Lightroom, darktable y Windows).
- [ ] Un `.xmp` junto al archivo de origen gana al XMP incrustado.
- [ ] Reindexar una biblioteca ya importada rellena estrellas, etiquetas y
      cámara, y una segunda vez no cambia nada; las estrellas puestas a mano
      no se pisan nunca.
- [ ] Estrellas en el visor, el panel Detalles y la barra de selección; teclas
      `0` a `5`; «Deshacer» devuelve los valores anteriores.
- [ ] El panel de filtros combina tipo, cámara, objetivo, fechas, valoración
      mínima, favoritas, álbum y etiqueta con el texto (pruebas en el núcleo
      de cada filtro y de combinaciones).
- [ ] El rango de fechas usa el día de `taken_local`.
- [ ] Búsqueda con filtros en menos de 100 ms con 10 000 fotos
      (`docs/rendimiento.md`).
- [ ] XMP mal formado o malicioso no rompe la importación (pruebas).
- [ ] Los originales no se modifican y no se crea ningún `.xmp` (prueba
      automática en `crates/mnemos-core/tests/originales_intactos.rs`).

## Fuera de alcance
- Escribir estrellas, descripciones o palabras clave en los archivos o en
  `.xmp` (ni ahora ni como opción).
- Marcar como rechazada, banderas y etiquetas de color de Lightroom.
- Etiquetas jerárquicas («Viajes/2024/Roma»), que van en el bloque 4 del plan.
- XMP de vídeos y XMP extendido de JPEG.
- Buscar por el texto de la descripción (el índice de búsqueda solo tiene ruta
  y etiquetas; añadirla pide rehacer la tabla FTS5 y va aparte).
- Ordenar por valoración, búsquedas guardadas y filtros en Explorar.
- Filtros por ubicación o ciudad (spec 026, lugares).
- Editar la descripción desde MNEMOS.

## Coordinación con otras specs
- **022 (cimientos)**: el filtro de fechas usa `taken_local` (migración 0016).
  La tarea 8 espera a que se fusione la tarea de hora local de la 022. Las
  tareas 2 a 7 no dependen de ella.
- **023 (pilas)**: si se fusiona antes, los resultados filtrados siguen su regla
  para la búsqueda; las estrellas de la barra de selección se aplican como las
  demás acciones sobre pilas. Lo que falte se ajusta en la spec que llegue
  después.
- **Migración**: 0019 reservada; si otra spec cambia `photos` a la vez, se
  revisa el orden en `db.rs` al fusionar.

## Decisiones de Paulo (23 nov)
1. Las palabras clave del XMP se importan como etiquetas de MNEMOS, solo en la
   base de datos y sin tocar los originales.
2. Las miniaturas enseñan «★ N» pequeño solo en las fotos valoradas.

## Tareas (≤1 h cada una)
1. [ ] Capturas del diseño (estrellas en el visor y en Detalles, menú «Valorar»
       de la barra de selección, panel de filtros, pastillas, sin resultados),
       en claro y oscuro (`capturas/024-*.png`), para que Paulo las apruebe.
2. [x] `mnemos-imaging`: `xmp.rs` con `quick-xml` (JPEG, PNG, WebP, TIFF/RAW y
       `.xmp` suelto), valoración EXIF; pruebas con muestras reales y con XMP
       mal formado, enorme y con entidades; `cargo deny`. `read_xmp`,
       `read_xmp_sidecar`, `parse_xmp` y `exif_rating`; `quick-xml` 0.42 sin
       funciones opcionales (ya estaba en el árbol por Tauri). Las pruebas
       usan paquetes como los de Lightroom y darktable dentro de archivos
       mínimos generados en la prueba.
3. [x] `mnemos-core`: migración 0019; el indexado lee una vez XMP incrustado,
       valoración EXIF y cámara (`metadata_at`), crea las etiquetas y rellena la
       descripción vacía; pruebas de reindexado y de valor manual. En
       `crates/mnemos-core/src/metadata.rs`: `FileMeta::of_file`, `apply` y
       `Library::read_pending_metadata`, que `index` llama al final y que la
       app llamará en segundo plano al abrir una biblioteca antigua (tarea 6).
       La importación ya lee el XMP incrustado, después de aplicar Takeout.
       Índice parcial `photos_metadata_pending` para encontrar las pendientes.
4. [x] `mnemos-core`: la importación busca `foto.ext.xmp` y `foto.xmp` en la
       carpeta de origen y le da prioridad; pruebas con Takeout a la vez,
       RAW+JPEG y fotos repetidas. `metadata::sidecar_of` (también `.XMP` en
       mayúsculas) y `FileMeta::with_sidecar`; el `.xmp` no se copia a la
       biblioteca. Takeout y repetidas ya quedan cubiertas por la tarea 3 (la
       descripción de Takeout se aplica antes y los metadatos se leen una vez).
5. [x] `mnemos-core`: `set_rating` por `Scope` y deshacer; `rating` en
       `GridPhoto` y en la información del visor; pruebas. Como el resto de
       acciones por lotes, `set_rating(ids, n)` recibe ids (la interfaz
       resuelve «Seleccionar todo» con `select_all`) y devuelve
       `Vec<PreviousRating>` para `restore_ratings`. Ignora la Papelera.
       `rating` en `GridPhoto` y `PhotoDetails`; `Error::InvalidRating`.
6. [ ] Tauri y visor: `read_pending_metadata` en segundo plano al abrir la
       biblioteca; comandos `valorar` y `deshacer_valoracion`, estrellas en
       la barra del visor y en Detalles, teclas `0` a `5`, aviso con
       «Deshacer»; pruebas de la interfaz.
7. [ ] Barra de selección y cuadrícula: «Valorar», teclas con selección,
       «★ N» en las miniaturas valoradas; pruebas.
8. [ ] `mnemos-core` (espera a la hora local de la 022): `SearchFilters` en
       `search_filter`, `filter_options`, índices si hacen falta; pruebas de
       cada filtro y de combinaciones; medida con 10 000 fotos en
       `docs/rendimiento.md`.
9. [ ] Tauri e interfaz: argumento `filtros` en la búsqueda, comando
       `opciones_filtros`, botón y panel de filtros, pastillas, `Esc`, sin
       resultados; pruebas de la interfaz.
10. [ ] `tests/originales_intactos.rs` (importar con `.xmp`, reindexar, valorar
        y filtrar sin tocar originales ni crear `.xmp`) y sección «Filtros y
        estrellas» en `docs/arquitectura.md`.
11. [ ] Probarlo en la app real con la biblioteca del disco (Paulo).
