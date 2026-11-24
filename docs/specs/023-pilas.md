# 023 · Pilas

- Estado: en curso (spec aprobada el 24 nov; capturas del diseño esperando aprobación)
- Fase: F2 (0.5 beta)

## Qué y por qué
Cuando disparo en RAW + JPEG, hago una ráfaga o descargo de Google Fotos una
foto y su versión editada, la cuadrícula se llena de fotos casi iguales que
son la misma toma. Quiero verlas como **una sola tarjeta** (una pila), con la
foto que elija delante y el número de fotos que hay detrás, y poder abrirla
para ver las demás.

Una pila solo agrupa en la base de datos: no mueve, no renombra y no borra
ningún archivo (regla 1 de CLAUDE.md). Desapilar deja todo como estaba.

## Comportamiento

### Qué es una pila
- Un grupo de 2 o más fotos (o vídeos) de la biblioteca con una **portada**:
  la foto que representa a la pila en la cuadrícula.
- Una foto está como mucho en una pila.
- Todas las fotos de una pila están siempre en el mismo sitio: o todas en la
  línea de tiempo, o todas archivadas, o todas en la Papelera. Favoritos,
  álbumes y etiquetas sí pueden ser distintos en cada foto.
- Cada pila recuerda de dónde salió, para enseñarlo en la tarjeta y en
  Detalles: «RAW + JPEG», «Editada», «Ráfaga» o «Pila» (hecha a mano).

### Agrupación automática
Se hace al importar, con `mnemos index` y una vez al abrir una biblioteca que
tenga fotos sin examinar (las de antes de esta spec). Solo mira la base de
datos y, para las ráfagas, la cámara del EXIF de las candidatas. Cada foto se
examina **una sola vez** (como el `.json` de Takeout en la spec 007): si el
usuario desapila, nada vuelve a apilar esas fotos solo.

Las reglas, en este orden (una foto que entra en una pila ya no se mira para
las siguientes):

1. **RAW + JPEG.** Un archivo RAW (la lista `RAW_EXTENSIONS` de
   `mnemos-imaging`, la misma de la colección RAW de Explorar) y una foto que
   no es RAW (JPEG, HEIC, PNG…) en la misma carpeta de `Originales/`, con el
   mismo nombre sin extensión (sin distinguir mayúsculas) y, si las dos tienen
   fecha de captura, con menos de 2 s de diferencia. La comprobación de la
   fecha evita juntar `IMG_0001.CR2` y `IMG_0001.JPG` de dos cámaras distintas
   que caen en el mismo mes. Portada: la foto que no es RAW.
2. **Editada de Google Fotos.** Una foto cuyo nombre sin extensión termina en
   `-editada`, `-editado` o `-edited` (los mismos sufijos que ya conoce
   `takeout.rs`) y otra en la misma carpeta con el mismo nombre sin el sufijo.
   Portada: la editada, que es la que se ve en Google Fotos. Si la original
   ya está en una pila RAW + JPEG, la editada entra en esa pila y pasa a ser
   su portada.
3. **Ráfaga por el nombre.** Fotos de la misma carpeta con el mismo trozo
   `BURST` seguido de cifras en el nombre (`00000IMG_00000_BURST20190312123456789_COVER.jpg`,
   `00001IMG_00001_BURST20190312123456789.jpg`, de los Pixel y otros Android).
   Portada: la que lleva `_COVER`; si ninguna, la primera por nombre.
4. **Ráfaga por la cámara.** Fotos de la misma carpeta, de la misma cámara
   (marca y modelo del EXIF), cuyo nombre acaba en un número de archivo
   seguido (`IMG_4501`, `IMG_4502`, `IMG_4503`; `DSC01234`, `DSC01235`) y con
   0 o 1 s de diferencia entre cada una y la siguiente. Hacen falta al menos
   2 fotos. Así se recogen también las ráfagas de iPhone, que no llevan marca
   en el nombre. Portada: la primera.

Nunca se apilan solas:
- fotos sin fecha de captura en la regla 4;
- vídeos (las Live Photos van en la spec 025);
- fotos que el usuario sacó de una pila o que estaban en una pila que
  desapiló;
- fotos archivadas con fotos sin archivar, ni fotos de la Papelera.

### Guardar como copia (decidido por Paulo el 23 nov)
- La copia que crea «Guardar como copia» (spec 009) se apila con su original,
  con la copia de portada. Si el original no estaba en una pila, se crea una
  de tipo «Editada»; si ya lo estaba, la copia entra en esa pila.
- Va en la misma transacción que la copia. «Deshacer» o desapilar dejan la
  copia suelta, como hoy.

Una foto importada más tarde se une a una pila que ya existe si cumple su
regla (por ejemplo, el RAW que se importa días después de su JPEG).

### Cuadrícula
- Una pila ocupa una sola casilla: la miniatura de su portada y, arriba a la
  derecha, una insignia con un icono de pila y el número de fotos («3»).
- La fecha, el día y el orden de la tarjeta son los de la portada.
- La barra de meses y las cabeceras de día cuentan tarjetas, no fotos, para
  que el alto reservado cuadre con lo que se pinta.
- Al seleccionar, la barra dice cuántas tarjetas y, si alguna es una pila,
  cuántas fotos hay en total: «3 seleccionadas (7 fotos)».

### Visor
- Al abrir una pila desde la cuadrícula se ve su portada y, abajo, una **tira**
  con las miniaturas de todas sus fotos, la portada marcada. Pulsar una
  miniatura de la tira la enseña en grande. `Mayús` + `←` / `→` recorre la
  tira; `←` / `→` siguen pasando de tarjeta en tarjeta, como hoy.
- La tira se puede ocultar (botón o tecla `S`); se recuerda mientras la app
  está abierta.
- En la barra superior, un botón **Abrir pila** lleva a la vista de la pila.
- El panel Detalles de una foto apilada dice «En una pila de 3 (RAW + JPEG)»
  con un enlace a la vista de la pila.

### Vista de la pila («abrirla»)
- Se llega con **Abrir pila** desde el visor o pulsando la insignia del número
  en la cuadrícula.
- Es la cuadrícula de siempre con las fotos de la pila (portada primero y
  luego por fecha), con una cabecera: flecha para volver, «Pila · 3 fotos» y
  su origen, y un menú con **Desapilar**.
- Aquí **cada acción afecta solo a las fotos que se seleccionan o se ven**:
  es el sitio para actuar sobre una foto de la pila.
- **Usar como portada**: con una foto seleccionada en la barra de selección,
  o desde el visor abierto en esta vista. Aviso con «Deshacer».

### Acciones sobre una pila (decidido por Paulo el 23 nov)
Fuera de la vista de la pila, **una pila se trata como una sola cosa y las
acciones afectan a todas sus fotos**. Vale para la barra de selección, el menú
contextual, las teclas y la barra del visor abierto desde la cuadrícula:

| Acción | Fuera de la pila (tarjeta) | Dentro de la pila (una foto) |
|---|---|---|
| Favorita | Todas las fotos | Solo esa foto |
| Archivar / desarchivar | Toda la pila | La foto sale de la pila y se archiva sola |
| Enviar a la Papelera | Toda la pila | La foto sale de la pila y va sola a la Papelera |
| Añadir a un álbum / quitar | Todas las fotos | Solo esa foto |
| Etiquetar / quitar etiqueta | Todas las fotos | Solo esa foto |
| Exportar (spec 008 y 017) | Todas las fotos | Solo esa foto |
| Valorar (spec 024) | Todas las fotos | Solo esa foto |

Los avisos lo dicen: «Pila de 3 fotos añadida a Favoritos», «2 pilas (7 fotos)
enviadas a la Papelera», siempre con «Deshacer». Deshacer devuelve también la
pila como estaba (por ejemplo, una foto que salió de la pila al archivarla
vuelve a su pila y a su sitio de portada).

Siempre son de una sola foto, la que se ve, estés donde estés:
- Editar (spec 009), fecha a mano y Detalles (spec 011).
- Copiar al portapapeles y Descargar (spec 020): copian o guardan la foto
  que se ve; desde una tarjeta de la cuadrícula, la portada.
- Ver las copias idénticas (spec 010).

Si una pila se queda con una sola foto (porque las demás salieron), deja de
ser pila y esa foto vuelve a ser una foto suelta. Si sale la portada, la
siguiente foto de la pila por fecha pasa a ser la portada.

### Apilar y desapilar a mano (barra de selección)
- **Apilar**: con 2 o más tarjetas seleccionadas. Las fotos sueltas y las
  pilas seleccionadas se juntan en una sola pila de tipo «Pila». La portada es
  la de la primera pila seleccionada en el orden de la cuadrícula o, si no hay
  ninguna, la primera foto. Desactivado, con una pista, si se mezclan
  archivadas y sin archivar (en la búsqueda, que enseña las dos).
- **Desapilar**: con alguna pila seleccionada. Sus fotos vuelven a ser
  sueltas y la agrupación automática ya no las toca.
- Los dos con aviso y «Deshacer».
- No están en la Papelera ni en Duplicados.

### Vistas y búsqueda
- **Fotos, Archivo y Papelera**: una tarjeta por pila (la portada). En la
  Papelera, «Restaurar» y «Eliminar para siempre» actúan sobre toda la pila;
  la confirmación dice el número de fotos («Se eliminarán 3 fotos para
  siempre»). El vaciado automático de los 30 días también.
- **Favoritos, álbumes, etiquetas, colecciones y recuerdos de Explorar**:
  sale la tarjeta de la pila (portada y número) si **alguna** de sus fotos
  cumple la condición de la vista. Con las acciones a toda la pila, lo normal
  es que la cumplan todas; si solo una foto está en un álbum, el álbum enseña
  la pila igual y al abrirla se ven todas. Los recuentos de las tarjetas de
  álbumes, etiquetas y colecciones cuentan tarjetas.
- **Búsqueda** (spec 003 y, cuando llegue, los filtros de la 024): la misma
  regla. Si cualquier foto de la pila coincide con el texto (por ejemplo,
  `IMG_0001` encuentra el RAW y el JPEG), sale una sola tarjeta con la
  portada. La búsqueda sigue incluyendo las archivadas.
- **Duplicados** (spec 010): no cambia. Las copias exactas se comparan foto a
  foto aunque estén en pilas, y cada copia apilada lleva la marca «En una
  pila». Enviar a la Papelera una copia desde Duplicados es una acción de una
  sola foto: la saca de su pila.

## Datos

### Migración 0018
```sql
-- Pilas (spec 023). Solo cambia la base de datos: ningún archivo se mueve.
CREATE TABLE stacks (
    id          INTEGER PRIMARY KEY,
    -- Foto que se enseña en la cuadrícula. La mantiene el núcleo: siempre es
    -- una foto de la pila.
    cover_id    INTEGER NOT NULL,
    kind        TEXT    NOT NULL
                CHECK (kind IN ('raw_jpeg', 'editada', 'rafaga', 'manual')),
    created_at  INTEGER NOT NULL
);

ALTER TABLE photos ADD COLUMN stack_id INTEGER
    REFERENCES stacks (id) ON DELETE SET NULL;
-- 1 si la foto está en una pila y no es su portada: las vistas la esconden
-- con `stack_hidden = 0`. Copia de `stack_id` y `cover_id` para que las
-- consultas de la cuadrícula no tengan que unir tablas.
ALTER TABLE photos ADD COLUMN stack_hidden INTEGER NOT NULL DEFAULT 0
    CHECK (stack_hidden IN (0, 1));
-- Cuándo la miró la agrupación automática (segundos Unix). NULL: pendiente.
ALTER TABLE photos ADD COLUMN stack_auto_at INTEGER;
-- 1 si el usuario la sacó de una pila o desapiló la suya: la agrupación
-- automática no la vuelve a apilar.
ALTER TABLE photos ADD COLUMN stack_manual INTEGER NOT NULL DEFAULT 0
    CHECK (stack_manual IN (0, 1));

CREATE INDEX photos_stack ON photos (stack_id) WHERE stack_id IS NOT NULL;
```
- Se añade al final de la lista de `crates/mnemos-core/src/db.rs` en el orden
  en que se fusione (detrás de la 0016 y la 0017 de la spec 022 si ya están).
- Los índices `photos_timeline_*` no se rehacen: las fotos escondidas son
  pocas y SQLite puede seguir usando el índice y descartarlas. Si las medidas
  con 10 000 fotos dicen otra cosa, esta misma migración los rehace con
  `AND stack_hidden = 0`, sobre la columna de fecha que haya en ese momento
  (`taken_local` si la 0016 ya está fusionada).
- Las bibliotecas que ya existen quedan con todas las fotos sueltas y
  pendientes (`stack_auto_at IS NULL`): se agrupan la primera vez que se
  abren con esta versión o con `mnemos index`.

### Núcleo (`crates/mnemos-core/src/stacks.rs`)
- `auto_stack_pending()`: aplica las cuatro reglas a las fotos con
  `stack_auto_at IS NULL`, buscando pareja también entre las ya examinadas,
  y marca `stack_auto_at`. Lo llaman la importación (al final, en una
  transacción), `index()` y la app al abrir la biblioteca si quedan
  pendientes. Las reglas se comprueban en Rust sobre las filas de cada
  carpeta, no con SQL de texto. Para la regla 4 la cámara sale de la columna
  `camera` de la spec 024 si ya existe y, si no, de `camera_info` (spec 011)
  solo para las candidatas.
- `stack(ids) -> Undo`, `unstack(stack_ids) -> Undo`, `set_cover(photo_id)`,
  `stack_members(stack_id)` y `restore_stacks(Undo)`.
- `expand_stacks(ids) -> Vec<i64>`: de las tarjetas (portadas) a todas las
  fotos de sus pilas. Las acciones por lotes (favoritos, archivo, papelera,
  álbumes, etiquetas, exportar y valorar) reciben un `Alcance`: `Pila` (por
  defecto: expanden con `expand_stacks`) o `Foto` (vista de la pila: no
  expanden y, para archivar y papelera, sacan antes la foto de su pila).
- Cada cambio mantiene en la misma transacción `stack_hidden`, `cover_id` y la
  regla de «una pila de una foto deja de ser pila». Una prueba comprueba esas
  invariantes tras cada operación.
- `View::Stack(id)`: la vista de la pila, con el mismo cursor que el resto.
- `Filter` gana la regla de las vistas: `stack_hidden = 0` en Fotos, Archivo y
  Papelera, y «alguna foto de la pila cumple» en las demás vistas y la
  búsqueda (la condición de la vista se aplica a las fotos y se devuelve la
  portada de su pila). `GridPhoto` gana `stack: Option<{ id, photos, kind }>`.
- `Scope` (spec 006) devuelve tarjetas; las acciones expanden después.
- `delete_permanently` y el vaciado de los 30 días borran la fila de
  `stacks` cuando ya no le quedan fotos.
- Tauri: comandos `apilar`, `desapilar`, `usar_como_portada`, `pila` (fotos
  de una pila) y `deshacer_pila`; las acciones por lotes ganan el argumento
  `alcance` (`"pila"` o `"foto"`).
- Rendimiento: con 10 000 fotos y 1 000 pilas, la primera página y los meses
  de cada vista en menos de 100 ms, la búsqueda igual, y
  `auto_stack_pending` sobre una biblioteca entera en menos de 2 s. Medidas
  en `docs/rendimiento.md`.

## Casos límite
- **Disco desconectado** a mitad de apilar, desapilar o de una acción sobre
  pilas: todo va en una transacción; se aplica entero o no se aplica, con el
  error de siempre.
- **Disco desconectado durante la agrupación automática**: lo no hecho queda
  con `stack_auto_at IS NULL` y se retoma la próxima vez.
- **Portada desaparecida** del disco (spec 002): la tarjeta enseña el marcador
  de siempre; el resto de la pila se ve en la tira.
- **Archivo corrupto o formato no compatible** sin miniatura: puede estar en
  una pila; la tira enseña el marcador «Sin vista previa».
- **Nombres repetidos**: `free_name` (spec 001) añade ` (1)` a un nombre ya
  usado; `IMG_0001 (1).JPG` no se empareja con `IMG_0001.CR2`. Se prefiere no
  apilar a apilar mal.
- **RAW con dos JPEG** (`IMG_0001.CR2`, `IMG_0001.JPG`, `IMG_0001.HEIC`): las
  tres en la misma pila; portada, el JPEG.
- **Editada de una editada** (`foto-editada-editada.jpg`): entra en la pila
  de la original; portada, la última.
- **Fechas a mano** (spec 011) distintas dentro de una pila: la tarjeta va por
  la fecha de la portada; la vista de la pila enseña cada una con la suya.
- **Guardar como copia** de una foto que ya es una copia o que está en una
  pila: la copia nueva entra en esa misma pila y pasa a ser la portada.
- **Restaurar** desde la Papelera una foto que salió de su pila: vuelve
  suelta. Deshacer justo después sí la devuelve a la pila.
- **Pilas enormes** (una ráfaga de 200 fotos): la tira se desplaza y carga
  las miniaturas por partes, como la cuadrícula.
- **Rutas con caracteres especiales** en Windows, macOS y Linux: las reglas
  comparan nombres como texto Unicode de la ruta relativa, ya guardada en
  UTF-8 (regla 2); la interfaz solo maneja ids.

## Criterios de aceptación
- [ ] Importar `IMG_0001.CR2` + `IMG_0001.JPG`, una foto de Takeout con su
      `-editado`, una ráfaga `BURST…_COVER` y 3 fotos seguidas de la misma
      cámara en el mismo segundo crea cuatro pilas con la portada correcta
      (pruebas en el núcleo con `tests/fixtures/`).
- [ ] Dos `IMG_0001` de cámaras distintas con fechas lejanas no se apilan.
- [ ] Desapilar y volver a importar o reindexar no vuelve a apilar.
- [ ] Una biblioteca existente se agrupa al abrirla o con `mnemos index`, y
      una segunda vez no cambia nada.
- [ ] Fotos, Archivo y Papelera enseñan una tarjeta por pila; Favoritos,
      álbumes, etiquetas, Explorar y la búsqueda enseñan la pila si alguna
      foto cumple (pruebas en el núcleo).
- [ ] Cada acción de la tabla, fuera y dentro de la pila, hace lo que dice, y
      «Deshacer» deja fotos y pilas como estaban (pruebas en el núcleo).
- [ ] Las invariantes (`stack_hidden`, portada dentro de la pila, ninguna
      pila de una foto, mismo sitio para todas las fotos) se cumplen tras
      cada operación (prueba automática).
- [ ] Rendimiento con 10 000 fotos en `docs/rendimiento.md`.
- [ ] Paulo aprueba las capturas del diseño antes de programar la interfaz.
- [ ] Los originales no se modifican, mueven ni renombran al apilar,
      desapilar, cambiar la portada ni con la agrupación automática (prueba
      automática en `crates/mnemos-core/tests/originales_intactos.rs`).

## Fuera de alcance
- Ráfagas de iPhone por su `BurstUUID` de las notas del fabricante y pares
  por el identificador de contenido de Apple (la regla 4 recoge las ráfagas
  por número y segundo; las Live Photos son la spec 025).
- Apilar fotos parecidas automáticamente (spec 027 añadirá «Apilar» en
  «Parecidas»).
- Apilar entre carpetas distintas de `Originales/`.
- Ajustes para activar o desactivar cada regla (se añadirán si las pruebas
  reales apilan de más).
- Pilas dentro de pilas, o una foto en varias pilas.
- Escribir nada en los archivos o en `.xmp` sobre la pila.

## Coordinación con otras specs
- **022 (cimientos, PR #139)**: la cuadrícula pasa a agrupar por
  `taken_local`. La pila usa la fecha de su portada con la columna que haya;
  si la 0018 tiene que rehacer índices, lo hace sobre `taken_local`. La regla
  4 compara `taken_at` (instantes), que la 022 no cambia.
- **024 (filtros y estrellas, PR #140)**: añade `camera` (migración 0019),
  estrellas y filtros. La regla «alguna foto de la pila cumple» vale para los
  filtros, y «Valorar» sigue la tabla de acciones. La que se fusione segunda
  ajusta su parte: si la 024 llega antes, esta spec añade la regla de pilas a
  `SearchFilters` y la valoración; si llega después, la 024 aplica la regla
  de pilas a sus filtros y su acción. Queda por ver en las pruebas reales si
  el filtro «Tipo: RAW» debería enseñar el propio RAW en vez de la portada.
- **025 (Live Photos)**: el vídeo de una Live Photo no es una pila; va con su
  foto (`live_video_id`). Si una foto con Live Photo está en una pila, su
  vídeo la acompaña en todas las acciones.
- **027 (parecidas)**: su botón «Apilar» usa `stack(ids)` de esta spec.
- **Migración**: 0018 reservada.

## Decisiones de Paulo (23 nov)
1. Las acciones sobre una pila afectan a toda la pila, salvo al abrirla y
   actuar sobre una foto.
2. «Guardar como copia» (spec 009) apila la copia editada con su original,
   con la copia de portada.

## Tareas (≤1 h cada una)
1. [ ] Capturas del diseño (insignia en la cuadrícula, tira del visor, vista
       de la pila, «Apilar»/«Desapilar»/«Usar como portada», avisos, Papelera
       con una pila), en claro y oscuro (`capturas/023-*.png`), para que Paulo
       las apruebe.
2. [x] `mnemos-core`: migración 0018, `stacks.rs` con `stack`, `unstack`,
       `set_cover`, `stack_members`, deshacer y las invariantes; pruebas.
3. [ ] `mnemos-core`: reglas 1 y 2 (RAW + JPEG y editada) en
       `auto_stack_pending`, llamado desde la importación e `index()`;
       pruebas con nombres repetidos, mayúsculas y fechas lejanas.
4. [ ] `mnemos-core`: reglas 3 y 4 (ráfagas por nombre y por cámara y
       segundo); pruebas con nombres de Pixel, Samsung, iPhone y cámaras.
5. [ ] `mnemos-core`: `Filter` con la regla de pilas en todas las vistas y la
       búsqueda, `View::Stack`, `GridPhoto.stack`, meses y recuentos por
       tarjetas; medidas con 10 000 fotos en `docs/rendimiento.md`.
6. [ ] `mnemos-core`: `Alcance` y `expand_stacks` en favoritos, archivo,
       papelera (con eliminar para siempre y vaciado), álbumes, etiquetas y
       exportar; salir de la pila al archivar o tirar una sola foto;
       deshacer; pruebas de cada fila de la tabla.
7. [ ] `mnemos-core`: «Guardar como copia» apila la copia con su original
       (portada, la copia); pruebas con original suelto y ya apilado.
8. [ ] Tauri: comandos nuevos, argumento `alcance`, agrupación al abrir una
       biblioteca con fotos pendientes (en segundo plano); pruebas.
9. [ ] Interfaz: insignia en la cuadrícula, recuento «(7 fotos)» en la barra
       de selección, «Apilar» y «Desapilar», avisos; pruebas.
10. [ ] Interfaz: tira del visor, `Mayús` + flechas, `S`, «Abrir pila», línea
        en Detalles; pruebas.
11. [ ] Interfaz: vista de la pila con alcance «foto», «Usar como portada»,
        Duplicados con la marca «En una pila»; pruebas.
12. [ ] `tests/originales_intactos.rs` (apilar, desapilar, portada, agrupación
        automática y acciones sobre pilas sin tocar originales) y sección
        «Pilas» en `docs/arquitectura.md`.
13. [ ] Probarlo en la app real con la biblioteca del disco (Paulo).
