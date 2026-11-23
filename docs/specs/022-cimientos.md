# 022 · Cimientos de conservación

- Estado: borrador
- Fase: F2

## Qué y por qué
Quiero que mi colección aguante años en un disco que se desenchufa, cambia de
equipo y envejece. Hoy, si `mnemos.db` se estropea (un tirón del cable a
mitad de escritura, un sector dañado), pierdo álbumes, etiquetas, favoritos,
fechas a mano y ediciones a la vista, aunque las fotos sigan ahí. Tampoco
tengo forma de saber si algún original se ha dañado con el tiempo. Y si abro
la biblioteca en un equipo con otra zona horaria, las fotos de por la noche
cambian de día.

Esta spec junta tres cimientos que protegen todo lo que venga después:

1. **Copias automáticas de la base** en `.mnemos/copias/`, comprobación al
   abrir y **restaurar** una copia.
2. **Hora local de captura**: agrupar por la hora que marcaba la cámara, no por
   la zona del equipo que abre la biblioteca.
3. **Comprobar la biblioteca**: recalcular la huella BLAKE3 de cada original
   para encontrar archivos dañados, que faltan o que sobran.

Ninguna de las tres toca los originales (regla 1 de CLAUDE.md). La idea de las
copias y la comprobación viene del estudio de Immich (informe «Immich frente a
MNEMOS»); el código es propio.

Orden de trabajo: primero la hora local, porque cuanto antes se guarde, menos
fotos habrá que corregir; después las copias y por último la comprobación.

## Comportamiento

### 1. Hora local de captura

#### El problema de hoy
`taken_at` guarda segundos Unix. Una fecha sin zona (EXIF sin desfase, nombre
del archivo, fecha a mano) se pasa a UTC con la zona del equipo **al indexar**,
y todas las consultas la vuelven a pasar a hora local con la zona del equipo
**al mirar** (`strftime(..., 'unixepoch', 'localtime')`). Si los dos equipos
tienen la misma zona, cuadra; si no, la cuadrícula se desplaza. Y una foto con
desfase en el EXIF (`OffsetTimeOriginal`, la mayoría de móviles) se enseña con
la hora del equipo, no con la del sitio donde se hizo: una foto de las 23:30 en
Nueva York sale a las 05:30 del día siguiente en un equipo en Madrid.

#### Lo que se guarda
Migración **0016** con dos columnas nuevas en `photos` y sus gemelas para la
fecha automática de reserva de la spec 011:

| Columna | Qué es |
|---|---|
| `taken_local` | Hora de pared de la captura, en segundos como si fuera UTC (`2024-07-14 23:30:00` → `1721000000`). Entera, para que `strftime(..., 'unixepoch')` sin `'localtime'` dé el día y la hora correctos y se pueda indexar. |
| `taken_offset` | Desfase en minutos respecto a UTC (`-240`, `120`), o `NULL` si no se sabe. |
| `taken_auto_local`, `taken_auto_offset` | Lo mismo para la fecha automática que se guarda cuando hay fecha a mano (spec 011), para volver a ella al quitar la fecha a mano. |

`taken_at` sigue siendo el instante exacto (UTC) y no cambia de significado:
lo usan la Papelera, la importación y todo lo que compara instantes.

De dónde sale `taken_local` según la fuente de la fecha:

| Fuente | `taken_local` | `taken_offset` |
|---|---|---|
| EXIF con desfase | La hora del EXIF tal cual | El desfase del EXIF |
| EXIF sin desfase, nombre del archivo | La hora tal cual | `NULL` |
| Vídeo con `com.apple.quicktime.creationdate` (lleva desfase) | Su hora tal cual | Su desfase |
| Vídeo con solo `creation_time` (UTC) | `taken_at` pasado con la zona del equipo al indexar | `NULL` |
| `.json` del Takeout (UTC) | `taken_at` pasado con la zona del equipo al indexar | `NULL` |
| Fecha a mano (spec 011) | La hora escrita | `NULL` |
| Sin fecha de captura | La fecha de modificación pasada con la zona del equipo al indexar | `NULL` |

La reserva «con la zona del equipo al indexar» reproduce lo que se ve hoy,
pero se calcula una sola vez y se guarda: ya no cambia al abrir la biblioteca
en otro equipo. Rellenar también las fotos sin fecha de captura deja una sola
columna para ordenar y agrupar, con un índice; `taken_source = 'ninguna'`
sigue diciendo que no es una fecha de captura.

#### Lo que agrupa por la hora local
Todo lo que hoy usa `'localtime'` sobre la fecha de captura pasa a usar
`taken_local` sin zona:
- La cuadrícula (orden y días), las cabeceras de día y sus títulos
  (`day_titles` ya se guarda por día local: no cambia).
- La barra de meses.
- Seleccionar un día entero (`selection.rs`).
- La búsqueda por fecha («julio 2024», «2019»).
- Recuerdos de Explorar y los rangos `View::Range` (spec 021): «hoy» es el
  día del equipo; las fotos se comparan por su día local de captura.
- Álbumes, Favoritos, Archivo, Etiquetas y colecciones, que usan la misma
  línea de tiempo.
- El panel Detalles enseña la hora local y, si se sabe, el desfase:
  «14 jul 2024, 23:30 (UTC−04:00)».

La Papelera agrupa por fecha de envío (`trashed_at`), que es un instante del
propio equipo: no cambia.

Los índices de la línea de tiempo (`photos_timeline_*`, migraciones 0005,
0007 y 0014) se rehacen sobre `taken_local DESC, id DESC` con las mismas
condiciones parciales (`archived_at`, `favorite`, `trashed_at IS NULL`).

#### Bibliotecas que ya existen
- La migración 0016 rellena `taken_local` y `taken_auto_local` con la
  expresión de hoy (`unixepoch(taken_at, 'unixepoch', 'localtime')`, y la
  fecha de modificación si no hay fecha de captura), con la zona del equipo
  que aplica la migración. Así, nada se mueve el primer día.
- `mnemos index` y «Volver a indexar» (si existe en la app; si no, solo la
  línea de órdenes) recalculan `taken_local` y `taken_offset` desde los
  originales y los `.json` del Takeout, leyéndolos y sin escribir en ellos.
  Respetan la fecha a mano: solo cambian la automática de reserva.
- La importación y «Guardar como copia» (spec 009) rellenan las columnas
  nuevas; la copia hereda las del original.

### 2. Copias automáticas de la base

#### Cuándo se copia
- **Al abrir la biblioteca** (la app al elegirla o al arrancar, y las órdenes
  de `mnemos` que la abren), como mucho **una vez al día**: si ya existe la
  copia de hoy, no se hace otra.
- **Siempre antes de aplicar una migración**, aunque ya haya copia de hoy.
- La copia se hace con la API de copias de SQLite (`rusqlite`, función
  `backup`, que ya está en la dependencia: solo se activa su *feature*). Es
  coherente aunque otra parte de la app esté escribiendo.

`Library::open` se llama muchas veces (cada comando de Tauri abre su
conexión), así que la copia diaria y la comprobación van en una función
aparte que la app llama una vez al abrir la biblioteca y la línea de órdenes
una vez por orden. La copia de antes de migrar va dentro de `db::open`, que es
donde se migra.

#### Dónde y cómo se guarda
- En `.mnemos/copias/` (la carpeta ya se crea con la biblioteca y hoy está
  vacía):
  - Diarias: `mnemos-AAAA-MM-DD.db`.
  - De antes de migrar: `mnemos-AAAA-MM-DD-antes-v14.db` (el número es la
    versión del esquema copiada).
  - De antes de restaurar: `mnemos-AAAA-MM-DD-hhmmss-antes-de-restaurar.db`.
- Se escribe en `.mnemos/tmp/` y solo al terminar se renombra a su sitio:
  nunca queda una copia a medias con nombre de copia buena.
- Tras escribirla, `PRAGMA quick_check` sobre la copia. Si falla, se borra y
  se avisa (no se guarda una copia mala).
- Fechas en el día del equipo; el nombre solo sirve para ordenar y leer.

#### Cuántas se guardan · PENDIENTE DE PAULO
> **Pregunta abierta:** cuántas copias diarias guardar. Propuesta: las
> **7 diarias** más recientes, las **3 últimas de antes de migrar** y las
> **3 últimas de antes de restaurar**. Con 10 000 fotos la base ocupa unos
> 10–20 MB, así que 13 copias son unos 250 MB como mucho.
>
> Hasta que Paulo conteste **no se implementa el borrado de copias**: las
> tareas guardan todas. La tarea de retención (tarea 7) queda en espera.

Cuando se implemente, la limpieza solo borra archivos de `.mnemos/copias/`
con los nombres de arriba; nunca otros archivos ni nada fuera de esa carpeta.

#### Comprobación al abrir
- Al abrir la biblioteca (una vez, como la copia diaria), `PRAGMA quick_check`.
  Con 10 000 fotos tiene que tardar menos de 200 ms en el disco externo
  (se mide y se apunta en `docs/rendimiento.md`).
- **Si falla**, o si SQLite no puede abrir la base («file is not a database»,
  «database disk image is malformed»), la app no enseña la cuadrícula: enseña
  una pantalla «La base de datos de esta biblioteca está dañada. Tus fotos
  están bien: lo dañado son los álbumes, etiquetas y demás datos de MNEMOS.»
  con la lista de copias y el botón «Restaurar la copia más reciente», que
  propone la más reciente que pasa `quick_check`.
- La línea de órdenes dice lo mismo y sugiere `mnemos copias` y
  `mnemos restaurar`, con código de salida distinto de cero.
- Si la base dañada se puede abrir, la copia diaria **no** se hace ese día
  (no se guarda una copia de una base mala encima de las buenas).

### 3. Restaurar una copia

#### Línea de órdenes
- `mnemos copias <biblioteca>`: lista las copias de la más reciente a la más
  antigua, con fecha, tipo (diaria, antes de migrar, antes de restaurar),
  versión del esquema, tamaño y número de fotos.
- `mnemos restaurar <biblioteca> <copia>`: `<copia>` es el nombre del archivo
  tal como lo lista `mnemos copias`. Sin `--confirmar` solo dice qué haría
  (como `mnemos papelera vaciar`); con `--confirmar` restaura.

#### Ajustes
- Sección nueva **«Biblioteca»** en la ventana de Ajustes (spec 013), con el
  grupo «Copias de la base»: fecha de la última copia, número de copias y el
  botón «Ver copias».
- «Ver copias» abre una lista con la misma información que `mnemos copias`
  y un botón «Restaurar» en cada fila.
- «Restaurar» pide confirmación: «Se volverá a como estaba la biblioteca el
  14 nov a las 10:02. Los álbumes, etiquetas, favoritos y fechas que hayas
  cambiado después se perderán. Antes se guarda una copia de la base actual
  por si quieres deshacerlo.» Botones «Cancelar» y «Restaurar».

#### Qué hace restaurar
1. Comprueba la copia con `quick_check` y que su esquema no es más nuevo que
   el que entiende esta versión (si lo es, no restaura y lo dice).
2. Guarda la base actual como copia «antes de restaurar» (si se puede abrir).
3. Copia la copia elegida sobre la base abierta con la API de SQLite
   (`restore`), sin cerrar la app ni borrar `mnemos.db` a mano.
4. Aplica las migraciones que falten si la copia es de un esquema anterior
   (con su copia de antes de migrar, como siempre).
5. La app recarga todas las vistas (`cambios.svelte.ts`).

Restaurar solo cambia `mnemos.db`. Las miniaturas, las recetas de
`.mnemos/ediciones/` y los originales no se tocan. Las consecuencias se
describen en «Casos límite».

### 4. Comprobar la biblioteca

#### Núcleo
- `Library::verify` recorre las fotos y vídeos de la base (también los de la
  Papelera y los archivados), lee cada original y recalcula su BLAKE3. Al
  terminar devuelve tres listas:
  - **Dañados:** la huella no coincide con `content_hash`.
  - **Que faltan:** el archivo no está en su ruta.
  - **Que sobran:** archivos dentro de `Originales/` que no están en la base.
    No cuentan los ocultos ni los del sistema (`.DS_Store`, `Thumbs.db`,
    `desktop.ini`, `._*`).
- **No borra, mueve, renombra ni repara nada.** Solo lee y apunta.
- Migración **0017**: `verified_at` (segundos Unix de la última comprobación
  correcta de esa foto) y `verify_problem` (`'danada'`, `'falta'` o `NULL`).
  Las dos se guardan foto a foto, en lotes pequeños, para no perder lo hecho.
- **Cancelar y retomar:** se puede cancelar en cualquier momento. La próxima
  vez continúa con las fotos que no tienen comprobación de la vuelta en curso
  (la vuelta se guarda con su fecha de inicio en
  `.mnemos/comprobacion.json`). Desenchufar el disco a mitad cuenta como
  cancelar.
- Progreso: fotos comprobadas, total, bytes leídos y tiempo restante
  estimado. Lo que manda es la velocidad del disco: con 10 000 fotos y 50 GB
  en un HDD USB, del orden de 10 minutos.
- Una foto con `verify_problem` que en una vuelta posterior vuelve a cuadrar
  (se ha restaurado el archivo desde otra copia) queda limpia.

#### Línea de órdenes
`mnemos verificar <biblioteca>`: progreso en una línea y, al final, un resumen
con las tres listas (rutas relativas). Código de salida distinto de cero si
hay dañados o que faltan. `Ctrl+C` cancela y deja lo hecho guardado.

#### Ajustes
- En la sección «Biblioteca», grupo «Comprobar biblioteca»: «Lee todas las
  fotos y vídeos para encontrar archivos dañados. Puede tardar varios
  minutos.», la fecha de la última comprobación completa y su resultado
  («Todo bien» o «3 dañadas · 1 falta · 2 sobran»).
- Botón «Comprobar» que pasa a barra de progreso con «Cancelar». Si quedó una
  vuelta a medias: «Continuar» y «Empezar de nuevo».
- La comprobación sigue aunque se cierre la ventana de Ajustes; al volver se
  ve el progreso.
- Con dañadas o que faltan, «Ver en la cuadrícula» abre esas fotos con una
  cabecera como la de las colecciones de Explorar («Con problemas», «Ajustes»
  con flecha para volver). Ahí se pueden ver, exportar (las que no faltan) o
  enviar a la Papelera como cualquier otra.
- Las que sobran se listan en un desplegable con su ruta y un botón
  «Importar» que abre el diálogo de importar de siempre con `Originales/`.
  No se importan solas.

### 5. Diseño
Capturas de la sección «Biblioteca» de Ajustes (con copias, lista de copias,
confirmación de restaurar, comprobación en curso, resultado con problemas),
la pantalla de base dañada y la cabecera «Con problemas», en claro y oscuro,
en `/mnt/project-files/capturas/022-*.png` para que Paulo las apruebe antes de
programar la interfaz. Mismos tokens y componentes que Ajustes (spec 013) y
las colecciones (spec 021).

## Casos límite
- **Disco desconectado** a mitad de una copia: queda un archivo en
  `.mnemos/tmp/`, que se limpia al abrir; las copias buenas siguen. A mitad de
  restaurar: SQLite deja la base como estaba o restaurada, nunca a medias
  (transacción); se vuelve a comprobar al abrir. A mitad de comprobar: se
  retoma.
- **Disco lleno:** si no cabe la copia, se avisa y la biblioteca se abre igual.
  Antes de migrar, si no cabe la copia, **no se migra** y se dice por qué.
- **Disco protegido contra escritura:** no hay copia ni comprobación; se abre
  como siempre (el modo solo lectura es otra spec).
- **Restaurar una copia antigua:** las fotos importadas después no están en la
  base; sus archivos siguen en `Originales/` y «Comprobar biblioteca» las
  lista como «que sobran», con «Importar». Las recetas de edición siguen en
  disco y vuelven a verse al reindexar. Las fotos eliminadas para siempre
  después de la copia saldrán como «que faltan».
- **Hora local con fechas raras:** fotos de antes de 1970 (`taken_local`
  negativo) y desfases poco comunes (+05:45, +13:00) funcionan; desfases fuera
  de ±14:00 se tratan como sin desfase.
- **Cambio de hora (verano/invierno):** la reserva «con la zona del equipo»
  usa la regla de esa fecha, como hoy.
- **Una foto con EXIF de una cámara con la hora mal puesta:** se enseña la
  hora que marcaba; se corrige con la fecha a mano (spec 011).
- **Rutas con caracteres especiales** en las copias: los nombres son ASCII
  fijos; las rutas de las listas de la comprobación son las relativas de la
  base (`RelPath`), igual que en el resto.
- **Biblioteca abierta en dos sitios a la vez** (app y línea de órdenes): la
  copia es coherente gracias a la API de SQLite; restaurar con la app abierta
  desde la línea de órdenes avisa de que la app debe recargar (y la app,
  al detectar que la base cambió de versión de datos, recarga).
- **Videos y RAW grandes:** la comprobación lee en bloques; no carga archivos
  enteros en memoria.

## Criterios de aceptación
- [ ] Una foto con EXIF `2024-07-14 23:30:00` y desfase `-04:00` sale en el día
      14 de julio a las 23:30 con cualquier zona del equipo (prueba con `TZ`
      distintas).
- [ ] Una foto sin desfase sale con la hora que marcaba la cámara en cualquier
      zona, también después de reindexar en otra zona.
- [ ] Cuadrícula, cabeceras, barra de meses, selección por día, búsqueda por
      fecha, Recuerdos y rangos usan `taken_local` (pruebas en el núcleo).
- [ ] Tras la migración 0016, una biblioteca existente se ve igual que antes en
      el mismo equipo; `mnemos index` rellena el desfase sin escribir en los
      originales.
- [ ] La línea de tiempo sigue a 60 fps y las consultas de meses y páginas no
      empeoran con 10 000 fotos (`docs/rendimiento.md`).
- [ ] Al abrir se crea como mucho una copia diaria, y siempre una antes de
      migrar; ninguna copia a medias con nombre de copia buena.
- [ ] Una base dañada a propósito se detecta al abrir y se restaura desde la
      app y desde `mnemos restaurar`.
- [ ] Restaurar guarda antes la base actual y aplica las migraciones que
      falten.
- [ ] `mnemos verificar` y «Comprobar biblioteca» encuentran un original
      alterado, uno borrado y uno añadido a mano; se pueden cancelar y
      retomar.
- [ ] Ajustes enseña copias y comprobación según el diseño aprobado.
- [ ] Los originales no se modifican (prueba automática ampliada a copias,
      restauración y comprobación).
- [ ] Retención de copias: según lo que decida Paulo (pendiente).

## Fuera de alcance
- Copias de los originales o de las miniaturas a otro disco (una spec de
  copias de seguridad completas, más adelante).
- Reparar originales dañados: MNEMOS solo avisa.
- Averiguar la zona horaria por el GPS de la foto.
- Corregir la hora de muchas fotos a la vez (desplazar ±N horas).
- `thumbhash` y vista previa en disco para fotos grandes: solo si la prueba de
  10 000 fotos reales lo pide (bloque 0 del plan), en su propia spec o como
  tarea añadida aquí.
- Modo solo lectura.

## Tareas (≤1 h cada una)
Hora local
1. [ ] Migración 0016 (`taken_local`, `taken_offset`, `taken_auto_local`,
       `taken_auto_offset`, relleno inicial e índices de la línea de tiempo
       rehechos) y escritura de las columnas en importar, indexar, fecha a
       mano y «Guardar como copia».
2. [ ] Consultas de la línea de tiempo, meses, selección por día y búsqueda por
       fecha con `taken_local`; medidas con 10 000 fotos.
3. [ ] Recuerdos, rangos y panel Detalles (hora local y desfase) con
       `taken_local`; pruebas con varias `TZ`.
4. [ ] `mnemos index` recalcula hora local y desfase en bibliotecas ya
       importadas; prueba de originales intactos.

Copias
5. [ ] Núcleo: copia diaria al abrir y copia antes de migrar (escritura en
       `tmp/`, `quick_check` de la copia, nombres), sin borrar ninguna.
6. [ ] Núcleo: `quick_check` al abrir y error claro de base dañada; pantalla de
       base dañada en la app.
7. [ ] *(En espera de Paulo)* Retención de copias.
8. [ ] Núcleo y línea de órdenes: listar copias y restaurar (`mnemos copias`,
       `mnemos restaurar … --confirmar`).

Comprobar
9. [ ] Núcleo: migración 0017 y `verify` con cancelar, retomar y las tres
       listas.
10. [ ] Línea de órdenes: `mnemos verificar` con progreso y resumen.

Interfaz
11. [ ] Diseño: capturas en claro y oscuro para aprobar.
12. [ ] Ajustes, sección «Biblioteca»: copias, lista y restaurar.
13. [ ] Ajustes: comprobar biblioteca con progreso y resultado; vista «Con
        problemas».

Cierre
14. [ ] Prueba de originales intactos ampliada (copias, restauración,
        comprobación) y sección en `docs/arquitectura.md`.
15. [ ] Prueba de Paulo con el disco real: hora local tras reindexar, copia
        diaria, restaurar y comprobar la biblioteca `MNEMOS/`.
