# 016 · Etiquetas

- Estado: borrador
- Fase: F2

## Qué y por qué
Quiero ponerles palabras a mis fotos («Playa», «Abuela», «Recibos») para
encontrarlas luego sin depender del nombre del archivo ni de la carpeta. Una
foto puede tener varias etiquetas y una etiqueta, muchas fotos. A diferencia
de un álbum, una etiqueta no tiene orden ni portada elegida: sirve para
clasificar y buscar.

Es una de las tres specs de F2 que quedaban (álbumes, etiquetas y papelera).
Las tablas `tags` y `photo_tags`, y la columna `tags` del índice de búsqueda,
existen desde la migración 0001 y siguen vacías.

## Comportamiento

### Pantalla Etiquetas
- **Dónde:** la entrada «Etiquetas» del menú lateral, hoy desactivada, abre la
  pantalla.
- **Contenido:** las etiquetas en una cuadrícula de tarjetas, por orden
  alfabético (sin distinguir mayúsculas ni acentos). Cada tarjeta lleva la
  miniatura de la foto más reciente de la etiqueta, el nombre y «N fotos».
- **Arriba:** el título «Etiquetas», el número de etiquetas y un botón
  «Nueva etiqueta» que abre un campo en línea para escribir el nombre.
- **Sin etiquetas:** un mensaje que explica cómo crearlas: «Selecciona fotos
  y pulsa Etiquetar, o crea una con Nueva etiqueta».
- **Menú de cada tarjeta** (botón de tres puntos al pasar el ratón, o clic
  derecho):
  - **Renombrar:** el nombre se vuelve editable en la tarjeta. `Intro` guarda
    y `Esc` cancela.
  - **Borrar:** pide confirmación: «¿Borrar la etiqueta «Playa»? Se quitará de
    N fotos. Las fotos no se borran.» Tras borrar, aviso de 8 s con
    «Deshacer», que la recrea con las mismas fotos.

### Fotos de una etiqueta
- Al pulsar una tarjeta se ve la cuadrícula de siempre con las fotos de esa
  etiqueta: agrupadas por día, con la barra de meses y el visor recorriendo
  solo esas fotos.
- Encima, una cabecera con una flecha para volver a Etiquetas, el nombre y
  «N fotos», y el mismo menú de la tarjeta (renombrar, borrar).
- Incluye las fotos archivadas, como Favoritos y la búsqueda, con la marca de
  archivada que ya tienen.
- La selección funciona igual que en Fotos (clic, `Ctrl`, `Mayús`, `Ctrl` +
  `A`, por día). En la barra de selección aparece además «Quitar de la
  etiqueta».

### Etiquetar varias fotos (barra de selección)
- Nuevo botón **Etiquetar** en la barra de selección, junto a Favoritos y
  Archivar.
- Abre un desplegable con un campo «Buscar o crear etiqueta» y la lista de
  etiquetas filtrada por lo escrito. Cada etiqueta lleva una casilla:
  - marcada si todas las fotos seleccionadas la tienen;
  - con un guion si solo la tienen algunas;
  - vacía si no la tiene ninguna.
- Pulsar una etiqueta vacía o con guion se la pone a todas; pulsar una marcada
  se la quita a todas. El cambio se guarda al momento.
- Si lo escrito no coincide con ninguna etiqueta, la primera fila es
  «Crear «texto»». `Intro` elige la primera fila.
- Al cerrar el desplegable, aviso de 8 s con lo hecho («Etiqueta «Playa»
  añadida a 12 fotos») y «Deshacer», que devuelve cada foto a como estaba.

### Etiquetas de una foto (visor)
- Nueva sección **Etiquetas** en el panel Detalles, debajo de Fecha.
- Cada etiqueta es una pastilla con su nombre (al pulsarla se abre la
  etiqueta) y una cruz para quitarla.
- «Añadir etiqueta» abre un campo con las mismas sugerencias y la opción de
  crear. Con el panel abierto, la tecla `T` lleva al campo.

### Búsqueda
- Las palabras de la búsqueda (spec 003) también se buscan en los nombres de
  las etiquetas de cada foto, con las mismas reglas: sin distinguir
  mayúsculas ni acentos y como inicio de palabra. `playa` encuentra las fotos
  con la etiqueta «Playa» aunque su ruta no lo diga, y `playa 2024` las de
  2024.
- La pista de «No hay fotos para «texto»» pasa a decir que se busca por
  nombre, carpeta, etiqueta o fecha.

### Nombres
- Se quitan los espacios del principio y del final y los repetidos en medio.
  Entre 1 y 50 caracteres; sin caracteres de control.
- No puede haber dos etiquetas con el mismo nombre sin distinguir mayúsculas
  («playa» y «Playa» son la misma). Los acentos sí cuentan: «Malaga» y
  «Málaga» serían dos etiquetas distintas, aunque la búsqueda encuentra las
  dos.
- Al crear una etiqueta que ya existe, se usa la existente.
- Al renombrar con el nombre de otra etiqueta, se pregunta: «Ya existe
  «Playa». ¿Unir las dos?». Al unirlas, las fotos de la primera pasan a la
  segunda y la primera desaparece.

### Datos
- Las etiquetas solo viven en la base de datos de la biblioteca
  (`.mnemos/mnemos.db`). Nunca se escriben en los archivos de imagen.
- **Guardar como copia** (spec 009): la copia lleva las mismas etiquetas que
  la foto, como ya lleva sus favoritos y sus metadatos.
- La migración 0013 añade el índice de `photo_tags` por etiqueta y rehace la
  columna `tags` del índice de búsqueda a partir de `photo_tags`.

## Casos límite
- Disco desconectado a mitad de una operación: cada cambio va en una
  transacción, así que se aplica entero o no se aplica, y se ve un error
  claro.
- Etiquetar 10 000 fotos de golpe (`Ctrl` + `A` y Etiquetar): menos de 300 ms
  con el índice de búsqueda incluido.
- Una foto desaparecida del disco conserva sus etiquetas y sigue saliendo en
  su etiqueta, como en la cuadrícula.
- Una etiqueta que se queda sin fotos (se le quitan todas) no se borra sola:
  sigue en la pantalla con «0 fotos» y una tarjeta sin miniatura.
- Nombres con emojis, comillas, barras u operadores de FTS5 (`AND`, `"`,
  `*`): se guardan tal cual y nunca rompen la búsqueda.
- Rutas con caracteres especiales en Windows, macOS y Linux: la interfaz solo
  maneja ids.
- Cientos de etiquetas: la pantalla y el desplegable siguen respondiendo al
  momento (menos de 50 ms con 500 etiquetas y 10 000 fotos).

## Criterios de aceptación
- [ ] Crear, renombrar, unir y borrar etiquetas, con la regla de nombres
  (prueba automática).
- [ ] Poner y quitar una etiqueta a varias fotos en una transacción devuelve
  las fotos que han cambiado, y «Deshacer» las deja como estaban (prueba
  automática).
- [ ] Buscar `playa` encuentra una foto con la etiqueta «Playa» y deja de
  encontrarla al quitársela, renombrar la etiqueta o borrarla (prueba
  automática).
- [ ] La vista de una etiqueta pagina con el mismo cursor que la cuadrícula e
  incluye las archivadas (prueba automática).
- [ ] Guardar como copia conserva las etiquetas (prueba automática).
- [ ] Etiquetar 10 000 fotos sintéticas tarda menos de 300 ms (en
  `docs/rendimiento.md`).
- [ ] Paulo aprueba las capturas del diseño antes de programar la interfaz.
- [ ] Los originales no se modifican (prueba automática).

## Fuera de alcance
- Escribir las etiquetas en los archivos (XMP, IPTC) o leerlas de ellos.
- Etiquetas anidadas («Viajes / Asturias»), colores o iconos por etiqueta.
- Etiquetas automáticas por contenido (caras, objetos, lugares).
- Combinar etiquetas con operadores («Playa y no Abuela»); se puede escribir
  `playa abuela` para las que tienen las dos.
- Sugerencias de etiquetas mientras se escribe en el campo de búsqueda.

## Tareas (≤1 h cada una)
1. [x] Migración `0013_etiquetas.sql` (índice de `photo_tags` por etiqueta y
   reconstrucción de `photo_search.tags`) y núcleo: crear, renombrar, unir,
   borrar y listar con recuento y foto de portada. Cada cambio rehace
   `photo_search.tags` de las fotos afectadas. Con pruebas.
2. [ ] Núcleo: poner y quitar una etiqueta a varias fotos (devuelve las que
   cambian, para «Deshacer»), etiquetas de una foto y estado de cada etiqueta
   en una selección (todas, algunas, ninguna). La copia hereda las etiquetas.
   Medición con 10 000 fotos.
3. [ ] Núcleo: fotos de una etiqueta (meses, páginas y `Scope` para la
   selección) y la búsqueda por etiqueta. Comando `mnemos etiquetas` en la CLI.
4. [ ] Comandos de Tauri y sus envoltorios en `biblioteca.ts`.
5. [ ] Capturas del diseño (pantalla, fotos de una etiqueta, desplegable de la
   barra de selección y sección del visor) para que Paulo las apruebe.
6. [ ] Pantalla Etiquetas y fotos de una etiqueta; activar la entrada del menú
   lateral.
7. [ ] Botón Etiquetar en la barra de selección, con su desplegable,
   «Quitar de la etiqueta» y el aviso con «Deshacer».
8. [ ] Sección Etiquetas en el panel Detalles del visor y tecla `T`.
9. [ ] Prueba de que ninguna acción de etiquetas cambia los originales,
   `docs/arquitectura.md` y `docs/progreso.md`.
