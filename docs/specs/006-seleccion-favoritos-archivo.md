# 006 · Selección múltiple, favoritos, archivo y título del día

- Estado: en curso
- Fase: F2

## Qué y por qué
Quiero marcar varias fotos a la vez y hacer algo con ellas: guardar las
mejores en Favoritos y apartar las que no quiero ver en la línea de tiempo
(capturas, recibos) sin borrarlas. También quiero ponerle un nombre a un día,
como «Ruta del Cares», igual que en Google Fotos.

Es la primera spec de F2 porque álbumes, etiquetas y papelera actúan sobre una
selección de fotos y la necesitan hecha. Sale de la comparativa con Google
Fotos (puntos 1, 2 y 7 de la prioridad 1, y el título del día).

## Comportamiento

### Selección múltiple
- **Casilla:** al pasar el ratón por una miniatura aparece un círculo arriba a
  la izquierda. Al pulsarlo, la foto queda seleccionada: casilla azul marcada y
  miniatura algo reducida, con marco del color de acento.
- **Modo selección:** mientras haya al menos una foto seleccionada, un clic en
  cualquier miniatura la marca o la desmarca en vez de abrir el visor, y todas
  las casillas se ven sin pasar el ratón.
- **Teclado y ratón:**
  - `Ctrl` + clic (`Cmd` en macOS) marca o desmarca una foto sin abrirla, aunque
    no haya selección.
  - `Mayús` + clic selecciona todas las fotos entre la última marcada y esta,
    en el orden de la cuadrícula, aunque haya páginas sin cargar en medio.
  - `Ctrl` + `A` selecciona todas las fotos de la vista actual (la biblioteca,
    los resultados de una búsqueda, Favoritos o Archivo).
  - `Esc` quita la selección. Si no la hay, `Esc` hace lo de siempre (salir de
    la búsqueda).
- **Por día:** al pasar el ratón por la cabecera de un día aparece un círculo
  junto a la fecha que selecciona o deselecciona todas las fotos de ese día. Si
  todas están seleccionadas, el círculo se ve marcado.
- **Barra de selección:** sustituye a la barra superior mientras hay
  selección. A la izquierda, una cruz para cancelar y «N seleccionadas». A la
  derecha, las acciones de esta spec:
  - estrella: añadir a Favoritos (o quitar, si todas lo son ya);
  - archivar (o desarchivar, dentro de Archivo).
  Álbumes, etiquetas y papelera añadirán aquí sus botones en sus specs.
- La selección se mantiene al desplazarse por la cuadrícula y al cargar más
  páginas. Se pierde al cambiar de vista (de Fotos a Favoritos, al buscar o al
  salir de la búsqueda).
- La selección vive en la interfaz como una lista de ids; nunca se guarda en
  la base de datos.

### Favoritos
- **Marcar:** desde la barra de selección, desde el botón de estrella del visor
  o con la tecla `F` en el visor y con la selección activa. `F` es un
  interruptor: si la foto ya es favorita, deja de serlo.
- **Señal en la cuadrícula:** las favoritas llevan una estrella blanca pequeña
  abajo a la izquierda de la miniatura, con sombra para leerse sobre cualquier
  foto.
- **Vista Favoritos:** la entrada «Favoritos» del menú lateral se activa. Muestra
  las favoritas con la misma cuadrícula por días, barra de meses y visor que
  Fotos. Si no hay ninguna: «Aún no tienes favoritos. Pulsa la estrella o la
  tecla F en una foto para añadirla.»
- Una favorita archivada sigue saliendo en Favoritos.

### Archivo
- **Archivar:** desde la barra de selección, desde el visor (botón junto a la
  estrella) o con `Mayús` + `A`. La foto desaparece de la línea de tiempo de
  Fotos. En el visor se pasa a la foto siguiente.
- **Deshacer:** tras archivar o desarchivar aparece un aviso abajo, «N fotos
  archivadas · Deshacer», durante 8 segundos.
- **Vista Archivo:** entrada nueva «Archivo» en el menú lateral, entre
  Favoritos y Papelera, con el mismo icono de caja que usa Google Fotos. Es la
  misma cuadrícula filtrada. Si está vacía: «No hay nada archivado. Las fotos
  archivadas no salen en Fotos, pero siguen en tu biblioteca y en la búsqueda.»
- Dentro de Archivo, la acción de la barra y del visor es «Desarchivar», que
  devuelve la foto a Fotos.
- **Dónde siguen saliendo:** en la búsqueda, con un icono de caja pequeño en la
  miniatura, y en Favoritos. Más adelante, también en los álbumes.
- Archivar nunca mueve el archivo: solo cambia la base de datos.

### Título del día
- Al pasar el ratón por la cabecera de un día aparece «Añadir título» junto a
  la fecha. Al pulsarlo, se escribe en el sitio; `Intro` o salir del campo
  guarda y `Esc` cancela. Un título vacío lo borra.
- Se muestra en la cabecera a continuación de la fecha, más oscuro y en el mismo
  tamaño: «sáb, 12 oct 2024 · Ruta del Cares». Al pulsar el título se edita.
- Hasta 100 caracteres. Si no cabe en el ancho del día (días cortos que
  comparten fila), se corta con «…» y el título completo sale al pasar el
  ratón.
- El título es del día, no de las fotos: se ve en Fotos, en Favoritos, en
  Archivo y en los resultados de una búsqueda, siempre que ese día tenga fotos
  en la vista.
- Un día es la fecha en hora local tal como ya agrupa la cuadrícula
  (`"2024-10-12"`), y el título se guarda con esa clave.

### Datos
- Una migración nueva añade a `photos` las columnas `favorite` (0 o 1) y
  `archived_at` (segundos Unix o NULL), y crea la tabla `day_titles` (`day`
  TEXT clave primaria `AAAA-MM-DD`, `title` TEXT, `updated_at`).
- Los índices de la línea de tiempo se ajustan para que Fotos (sin archivadas),
  Favoritos y Archivo paginen igual de rápido que hoy.
- Todo cambio sobre una selección va en una sola transacción: o se aplica a
  todas las fotos o a ninguna.
- Nada se escribe en los archivos de imagen (ni EXIF ni XMP ni archivos al
  lado). Todo vive en `.mnemos/mnemos.db`, que viaja con el disco.

## Casos límite
- Disco desconectado a mitad de una acción: la transacción no se aplica, sale
  un error claro y la selección se conserva para reintentar.
- Seleccionar con `Ctrl` + `A` o por rango 10 000 fotos o más: los ids salen del
  núcleo con una consulta, no de las páginas cargadas, y marcar o archivar
  10 000 fotos tarda menos de 1 s.
- `Mayús` + clic sin una foto marcada antes: se comporta como `Ctrl` + clic.
- Fotos desaparecidas del disco: se pueden seleccionar, marcar y archivar como
  las demás.
- Favoritas en una selección mixta: la estrella de la barra marca todas; solo
  quita si todas ya eran favoritas.
- Archivar desde los resultados de una búsqueda: la foto sigue en los
  resultados, con el icono de caja.
- Archivar todas las fotos de un mes: el mes desaparece de la barra de meses
  de Fotos.
- Una foto movida o renombrada fuera de la aplicación pierde su favorito y su
  archivo al volver a indexar, porque pasa a ser otro registro. Recuperarlos
  por la huella (`content_hash`) queda para la spec de desaparecidas.
- La misma biblioteca abierta en un equipo con otra zona horaria: alguna foto
  cercana a medianoche puede cambiar de día, y el título se queda en su fecha.
- Títulos con emojis, comillas, saltos de línea pegados o texto de derecha a
  izquierda: se guardan tal cual, los saltos de línea se cambian por espacios y
  la interfaz nunca los interpreta como HTML.
- Rutas con caracteres especiales en Windows, macOS y Linux: la interfaz solo
  maneja ids.

## Criterios de aceptación
- [x] Marcar y desmarcar favoritos sobre una selección cambia exactamente esas
  fotos y nada más (prueba automática).
- [x] La línea de tiempo y la barra de meses de Fotos no incluyen fotos
  archivadas; Archivo solo incluye archivadas; Favoritos incluye favoritas
  archivadas o no (prueba automática).
- [x] La búsqueda devuelve las fotos archivadas (prueba automática).
- [ ] El rango de `Mayús` + clic incluye las fotos de páginas no cargadas, en
  el orden de la cuadrícula (prueba automática del núcleo y de la interfaz).
- [x] Si la escritura falla a mitad de una selección, ninguna foto cambia
  (prueba automática).
- [x] Favoritos, archivo y títulos sobreviven a cerrar la app y abrir la
  biblioteca desde otro equipo (prueba automática con copia de la biblioteca a
  otra ruta).
- [x] Marcar o archivar 10 000 fotos sintéticas tarda menos de 1 s y paginar
  Fotos, Favoritos y Archivo sigue por debajo de lo medido en
  `docs/rendimiento.md`.
- [ ] Deshacer devuelve las fotos a su estado anterior.
- [ ] Capturas de la barra de selección, la estrella, Archivo y el título del
  día aprobadas por Paulo antes de cerrar la interfaz.
- [ ] Los originales no se modifican: huella BLAKE3 y fecha de modificación
  iguales antes y después de cada acción (prueba automática).

## Fuera de alcance
- Álbumes, etiquetas y papelera: specs siguientes, que añaden sus botones a la
  barra de selección.
- Importar favoritos y archivo desde el JSON de Google Takeout: va con el resto
  de metadatos de Takeout.
- Buscar por favoritos o por título del día (`favoritos`, `Ruta del Cares`):
  con la búsqueda ampliada de F2.
- Selección arrastrando un rectángulo.
- Recuperar favoritos de fotos movidas fuera de la app.
- Escribir el favorito en el XMP o el EXIF de la foto: nunca, por la regla de
  originales intactos.

## Tareas (≤1 h cada una)
1. [x] Migración con `favorite`, `archived_at`, `day_titles` e índices, y su
   prueba de migración desde una biblioteca v6.
2. [x] Núcleo: marcar y desmarcar favoritos y archivar o desarchivar una lista
   de ids en una transacción, con pruebas (incluido el fallo a mitad).
3. [x] Núcleo: filtro de vista (Fotos sin archivadas, Favoritos, Archivo) en
   meses y páginas de la línea de tiempo, con la búsqueda incluyendo
   archivadas. Medición con 10 000 fotos.
4. [x] Núcleo: ids de un día, de toda la vista y de un rango entre dos fotos,
   para seleccionar sin cargar páginas.
5. [x] Núcleo: leer y guardar títulos de día, con pruebas de textos raros.
6. [x] Comandos de Tauri para las tareas 2 a 5.
7. [ ] Capturas del diseño (casillas, barra de selección, estrella, aviso de
   deshacer, título del día) en claro y oscuro para que Paulo las apruebe.
8. [ ] Estado de selección en la interfaz con pruebas: clic, `Ctrl`, `Mayús`,
   `Ctrl` + `A`, por día y `Esc`.
9. [ ] Casillas y cabeceras de día seleccionables en la cuadrícula.
10. [ ] Barra de selección con favoritos y archivar, y aviso con deshacer.
11. [ ] Estrella y archivar en el visor, teclas `F` y `Mayús` + `A`.
12. [ ] Vistas Favoritos y Archivo en el menú lateral, con sus mensajes vacíos.
13. [ ] Título del día editable en la cabecera.
14. [ ] Prueba de originales intactos y de biblioteca copiada a otra ruta;
   actualizar `docs/progreso.md`.
