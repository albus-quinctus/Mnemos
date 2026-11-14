# 019 · Vídeos

- Estado: en curso
- Fase: F2

## Qué y por qué
Mis vídeos del móvil y de Google Takeout tienen que estar en la biblioteca
junto a las fotos: verlos en la cuadrícula en su día, reproducirlos en el visor
y organizarlos igual (favoritos, archivo, álbumes, etiquetas, papelera). Hoy
MNEMOS los ignora al importar: el Takeout de Paulo trae 146 vídeos que no
entran, y 14 de ellos son los archivados que faltan (Google enseña 87
archivados y MNEMOS 73).

## Decisión pendiente: cómo se generan las miniaturas y qué se puede reproducir
MNEMOS no lee hoy ningún vídeo. Reproducir lo hace bien la vista web de cada
sistema (`<video>`), pero sacar un fotograma para la miniatura y entender los
códecs necesita un descodificador de vídeo, y en Rust puro no hay uno fiable
para H.264 ni HEVC. Tres caminos:

- **A. Solo la vista web (recomendada para empezar).** Sin binarios nuevos.
  El núcleo lee del contenedor (MP4, MOV, 3GP; Rust puro) la fecha, la
  duración, el tamaño y el giro. La miniatura la saca la propia interfaz: carga
  el vídeo en un `<video>` oculto, va al segundo 1 (o a la mitad si dura menos
  de 2 s), lo pinta en un `<canvas>` y lo manda al núcleo, que lo guarda como
  cualquier miniatura. Lo que se puede reproducir y lo que tiene miniatura
  coinciden: si la vista web no entiende el códec, el vídeo sale con una
  miniatura genérica y el visor lo dice. En la AppImage de Linux se activa
  `bundleMediaFramework` para llevar GStreamer dentro y no depender del
  sistema. Límites conocidos: HEVC (los vídeos de iPhone) en Linux y en
  Windows sin la extensión HEVC de Microsoft; la CLI no genera miniaturas de
  vídeo.
- **B. Llevar `ffmpeg` dentro.** Un ejecutable por sistema junto a la app.
  Miniaturas desde el núcleo y la CLI, cualquier códec, y la opción de hacer una
  copia H.264 para reproducir lo que la vista web no entiende (en
  `.mnemos/`, nunca tocando el original). Coste: de 25 a 80 MB más por sistema
  en el zip portátil (menos con una compilación propia, que hay que mantener en
  el CI) y una decisión de licencia (LGPL o GPL, compatibles con la
  GPL-3.0-or-later de MNEMOS).
- **C. A ahora y B solo si hace falta.** Empezar con A y decidir B en
  `docs/decisiones/` después de probar con los 146 vídeos reales en Linux y en
  Windows.

**Elegido por Paulo el 14 nov: C.** Esta spec describe A; B se decidirá en
`docs/decisiones/` solo si la prueba con los vídeos reales lo pide.

## Comportamiento
- **Formatos.** Se importan como vídeo: `mp4`, `m4v`, `mov`, `3gp`, `3g2`,
  `webm`, `mkv`, `avi`, `mts`, `m2ts`, `wmv`. Todos entran en la biblioteca y
  se conservan intactos; que se reproduzcan depende de la vista web (ver arriba).
- **Importar.** Igual que las fotos (spec 001): copia verificada con BLAKE3 en
  `Originales/AAAA/MM/`, sin tocar el origen, duplicados exactos por huella.
  El resumen de la importación los cuenta aparte: «120 fotos y 14 vídeos
  importados».
- **Fecha de captura**, de la más fiable a la menos, como en la spec 004 con el
  contenedor en el lugar del EXIF: fecha de creación del contenedor (`mvhd`,
  en UTC; se ignora si vale 0, la de 1904), nombre del archivo con fecha y hora
  (`VID_20190512_143000.mp4`), `.json` de Takeout, nombre con solo el día
  (`VID-20181215-WA0003.mp4`) y fecha de modificación. La fecha a mano de la
  spec 011 también vale para vídeos.
- **Takeout (spec 007).** El `.json` de un vídeo se busca igual que el de una
  foto (`VID_….mp4.supplemental-metadata.json` y sus variantes recortadas):
  descripción, ubicación, favorito, archivado y álbumes se aplican igual. Así
  entran los 14 archivados que faltan.
- **Cuadrícula.** El vídeo ocupa su sitio por fecha, con la proporción de su
  imagen ya girada (los móviles guardan en horizontal con una marca de giro de
  90°). Encima de la miniatura, abajo a la derecha, la duración («0:42»,
  «1:05:10») con un pequeño triángulo de reproducir, sobre una sombra para que
  se lea en fotos claras.
- **Visor.** Al abrir un vídeo empieza a reproducirse con sonido, con los
  controles nativos de la vista web (reproducir, barra de tiempo, volumen y
  pantalla completa). Al pasar a otro elemento o cerrar el visor se para. El
  volumen y el silencio se recuerdan durante la sesión.
  - Teclado: `Espacio` reproduce o pausa, `M` silencia, `Mayús` + `←`/`→`
    retrocede o avanza 5 s. `←`/`→` siguen pasando de elemento, como con las
    fotos.
  - La precarga de anterior y siguiente (spec 002) solo carga la miniatura de
    un vídeo, nunca el vídeo.
  - Si no se puede reproducir: la miniatura (o la genérica) con el aviso
    «Este vídeo no se puede reproducir en este equipo» y el botón «Abrir con
    otra aplicación», que lo abre con el programa del sistema (el original, sin
    copiarlo; MNEMOS no lo cambia).
  - Favorito, archivar, papelera, etiquetas, álbumes y Detalles funcionan igual
    que con una foto. «Editar» no aparece en los vídeos (y `E` no hace nada).
- **Detalles (spec 011).** Para un vídeo: fecha, nombre, duración, resolución
  (ya girada), fotogramas por segundo, códec de vídeo y de audio si se pueden
  leer, tamaño del archivo, ubicación y descripción del Takeout, y cámara si el
  contenedor la trae (`©mak`, `©mod`). Sin sección «Ediciones».
- **Exportar (specs 008 y 017).** Tamaño original: copia verificada, igual que
  una foto. «Tamaño reducido» no está disponible para vídeos: en el visor la
  opción sale desactivada con «Solo tamaño original»; en un lote se salta el
  vídeo con el motivo «Los vídeos solo se exportan en tamaño original», como ya
  pasa con HEIC.
- **Lo demás no cambia:** favoritos, archivo, álbumes, etiquetas, papelera
  (eliminar para siempre borra el vídeo como borraría una foto), duplicados,
  búsqueda por nombre, títulos del día y barra de meses tratan el vídeo como
  un elemento más. Los textos que dicen «fotos» se quedan como están salvo el
  resumen de la importación.

## Cambios técnicos
- **Migración 0015** (la siguiente a la 0014): `photos.media_kind TEXT NOT NULL
  DEFAULT 'foto'` (`'foto'` o `'video'`) y `photos.duration_ms INTEGER`. El
  indexado los rellena; las filas que ya hay son fotos.
- `mnemos-imaging` gana un lector de contenedores ISO (MP4, MOV, 3GP) en Rust
  puro, sin dependencias nuevas si se puede: duración, ancho, alto, giro,
  fecha, fps, códecs y cámara. Para `webm`, `mkv`, `avi`, `mts` y `wmv` solo
  se guarda lo que diga el archivo (puede quedarse sin duración ni tamaño hasta
  que la vista web lo cargue para la miniatura y lo devuelva).
- **Protocolo `mnemos://`:** nueva ruta `mnemos://localhost/video/<id>` que
  sirve el original **por trozos** (`Range`, respuesta 206): sin eso WebKit y
  AVFoundation no reproducen ni dejan saltar, y un vídeo de 2 GB no puede
  leerse entero en memoria. Lleva `Access-Control-Allow-Origin` para que el
  `<canvas>` de la miniatura no quede bloqueado. La CSP gana
  `media-src mnemos: http://mnemos.localhost`.
- **Miniaturas:** mismas carpeta y tamaño que las fotos
  (`.mnemos/miniaturas/v1/<ab>/<huella>.jpg`). La cola del núcleo se salta los
  vídeos; la interfaz pide los que no tienen miniatura y las va haciendo de una
  en una mientras la biblioteca está abierta, sin frenar la cuadrícula. Si
  falla, el vídeo usa la genérica. El fallo no se anota en la base de datos: se vuelve a intentar en
  la siguiente sesión, que puede ser en otro equipo con otros códecs (cambiado
  al implementarlo; antes decía `thumbnail_failures`).

## Casos límite
- Disco desconectado a mitad de reproducción: el visor enseña «El archivo no
  está en el disco», como con una foto desaparecida.
- Vídeos grandes (varios GB): la importación los copia y verifica por trozos;
  la barra de progreso cuenta archivos y puede quedarse quieta un rato en uno
  grande.
- Vídeo dañado o sin pistas de vídeo (solo audio): se importa, se queda con la
  miniatura genérica y el visor dice que no se puede reproducir.
- Live Photos de iPhone (`IMG_0001.HEIC` + `IMG_0001.MOV`) y fotos con
  movimiento de Pixel (`PXL_….MP.jpg`, vídeo dentro del JPEG): en esta spec el
  `.MOV` entra como vídeo aparte y el `.MP.jpg` como foto.
- Fecha del contenedor en UTC que en realidad es hora local (algunas cámaras):
  se guarda tal cual; se corrige con la fecha a mano.
- Rutas con espacios, tildes o caracteres especiales en Windows, macOS y
  Linux, también en la ruta `mnemos://localhost/video/<id>`.

## Criterios de aceptación
- [x] Importar una carpeta con fotos y vídeos copia los dos, verificados, en `Originales/AAAA/MM/` y el resumen los cuenta aparte (prueba automática).
- [x] La fecha de un vídeo sale del contenedor, del nombre o del `.json` de Takeout en ese orden, y un vídeo archivado en el Takeout entra archivado (prueba automática).
- [x] La migración 0015 deja las filas que ya había como fotos (prueba automática).
- [x] `mnemos://localhost/video/<id>` responde a `Range` con 206 y el trozo pedido sin leer el archivo entero (prueba automática).
- [x] En la cuadrícula el vídeo sale en su día, con su proporción girada y la duración.
- [x] En el visor se reproduce, se para al cambiar de elemento y responde a `Espacio`, `M` y `Mayús` + flechas; si no se puede reproducir sale el aviso con «Abrir con otra aplicación».
- [x] Exportar en tamaño original copia el vídeo verificado; en tamaño reducido se salta con su motivo (prueba automática).
- [x] Capturas de la cuadrícula, el visor, el aviso y Detalles aprobadas por Paulo.
- [x] Los originales no se modifican (prueba automática en `tests/originales_intactos.rs`).
- [ ] Probado en la app real con los vídeos del Takeout en Linux (Paulo); Google y MNEMOS cuentan los mismos archivados.

## Fuera de alcance
- Editar vídeos (recortar, girar, silenciar) y sacar un fotograma como foto.
- Unir Live Photos y fotos con movimiento con su vídeo, o reproducir el vídeo
  de dentro de un `.MP.jpg`.
- Convertir vídeos para que se reproduzcan donde no se puede (camino B).
- Exportar vídeos en tamaño reducido.
- Reproducción automática al pasar el ratón por la cuadrícula.
- Una vista o filtro «Solo vídeos» (puede ir después, como en Google Fotos).
- Cambiar los textos «fotos» por «elementos».

## Tareas (≤1 h cada una)
1. [x] Spec y decisión A/B/C (Paulo eligió C el 14 nov).
2. [x] Capturas del diseño: cuadrícula con duración, visor reproduciendo, aviso de no reproducible y Detalles, en claro y oscuro (aprobadas por Paulo el 14 nov; `capturas/019-*.png` en los archivos del proyecto).
3. [x] `mnemos-imaging`: lector de contenedores ISO (duración, tamaño, giro, fecha, fps, códecs, cámara) con pruebas sobre vídeos mínimos generados en `tests/fixtures/`.
4. [x] Migración 0015 y extensiones de vídeo en el indexado y la importación; fecha de captura con el contenedor; `.json` de Takeout para vídeos; resumen con fotos y vídeos aparte. Pruebas.
5. [x] Ruta `mnemos://localhost/video/<id>` con `Range`, cabecera CORS y CSP con `media-src`. Pruebas.
6. [x] Miniaturas de vídeo: la cola del núcleo se los salta, comandos para pedir los pendientes y guardar la miniatura que manda la interfaz y avisar de los fallos (sin anotarlos). Pruebas.
7. [x] Interfaz: generador de miniaturas en segundo plano (`<video>` + `<canvas>`) y miniatura genérica.
8. [x] Cuadrícula: duración con el icono de reproducir; `GridPhoto` lleva `media_kind` y `duration_ms`.
9. [x] Visor: reproductor, teclas, parar al cambiar, aviso de no reproducible con «Abrir con otra aplicación», sin «Editar».
10. [x] Detalles de un vídeo y exportación (sin tamaño reducido en el visor; motivo propio en el lote).
11. [x] AppImage con `bundleMediaFramework` y prueba en el CI de que lleva el descodificador H.264 de GStreamer.
12. [x] `tests/originales_intactos.rs` con vídeos, sección «Vídeos» en `docs/arquitectura.md` y `docs/progreso.md`.
13. [ ] Probarlo en la app real con el Takeout del HDD (Paulo).
