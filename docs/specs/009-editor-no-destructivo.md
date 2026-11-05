# 009 · Editor no destructivo: recortar y girar

- Estado: borrador
- Fase: F3

## Qué y por qué
Quiero enderezar una foto que salió torcida o quitarle un trozo que sobra sin
miedo a estropearla: el archivo original no se toca nunca y siempre puedo
volver a como estaba, aunque hayan pasado meses.

Es la primera spec del editor (prioridad 2 de la comparativa con Google
Fotos). Fija el formato de las recetas de edición que usarán las siguientes
(ajustes de luz y color, dibujo y anotaciones) y hace las dos operaciones que
más se usan: recortar y girar.

## Comportamiento

### Entrar y salir del editor
- En el visor, un botón «Editar» (icono de lápiz con regla) en la barra de
  arriba, o la tecla `E`. El visor pasa a modo edición sobre la misma foto, con
  el mismo fondo oscuro.
- Barra inferior del editor, de izquierda a derecha: girar 90° a la izquierda,
  voltear en horizontal, proporción del recorte (Libre, Original, Cuadrado,
  4:3, 3:2, 16:9, con su versión vertical) y «Restablecer». Arriba a la
  derecha, «Cancelar» y «Guardar», que despliega dos opciones: «Guardar como
  original» y «Guardar como copia» (ver abajo).
- `Esc` o «Cancelar» salen sin guardar. Si hay cambios sin guardar se pide
  confirmación: «¿Descartar los cambios?».
- Al guardar se vuelve al visor, que ya muestra la foto editada.

### Guardar como original o como copia
Petición de Paulo (2026-11-05).
- **Guardar como original:** solo queda la foto editada. Crea una versión
  nueva de la receta y la hace actual: en la cuadrícula, el visor, la búsqueda
  y los álbumes se ve solo la editada, con su fecha, su nombre y sus datos de
  siempre. El archivo de `Originales/` no se reescribe: queda oculto dentro de
  MNEMOS y se recupera con «Volver al original». Decidido por Paulo el
  2026-11-05 frente a sobrescribir el archivo, que habría sido una excepción a
  la regla 1 de `CLAUDE.md`.
- **Guardar como copia:** se conservan la original y la editada como dos fotos
  independientes.
  - La copia es un archivo nuevo, generado a partir del original y la receta,
    en la misma carpeta `Originales/AAAA/MM/` que el original.
  - Toma el nombre del original añadiendo un número: `IMG_1234.jpg` da
    `IMG_1234 (1).jpg`, y si ya existe, `IMG_1234 (2).jpg`, con la misma regla
    que la importación.
  - Toma sus datos: fecha y hora de captura (y su origen), fecha de
    modificación del archivo, descripción, ubicación y favorito. Así sale en la
    cuadrícula justo al lado del original. El EXIF del original se copia en la
    copia, con la orientación puesta a 1 y las dimensiones nuevas.
  - A partir de ahí es independiente: editarla, archivarla o borrarla no
    afecta al original, y viceversa. Sus propias ediciones son recetas sobre
    la copia.
  - Formato: JPEG de calidad 95 a tamaño completo si el original es JPEG, RAW,
    TIFF o WebP; PNG si el original es PNG (para no perder la transparencia).
  - El original y su receta no cambian. En el visor se pasa a la copia.

### Recortar
- Al entrar, la foto ocupa el área de trabajo con un marco de recorte que la
  cubre entera. El marco se arrastra por las esquinas y los lados; la zona que
  queda fuera se oscurece. Mientras se arrastra se ve la cuadrícula de tercios.
- Arrastrar dentro del marco mueve el recorte por la foto. El marco nunca sale
  de la imagen.
- Con una proporción elegida, las esquinas la mantienen. «Original» es la de
  la foto ya girada.
- Teclado: flechas para mover el marco 1 % y `Mayús` + flechas para 10 %.
- El recorte tiene como mínimo 32 × 32 píxeles del original.

### Girar y voltear
- «Girar» gira 90° a la izquierda cada vez (`R`; `Mayús` + `R` a la derecha).
  El recorte gira con la foto.
- «Voltear» refleja la foto en horizontal (como en un espejo).
- El giro se suma a la orientación EXIF que ya se respeta al mostrar la foto:
  se edita sobre la foto tal como se ve, no sobre como la guardó la cámara.

### Versiones y volver al original
- Cada «Guardar» añade una versión a la receta. Ninguna versión se borra al
  guardar otra.
- En el panel de detalles del visor aparece la sección «Ediciones» con la
  lista de versiones («Versión 3 · hoy, 18:20 · recorte y giro») y
  «Original». Pulsar una la convierte en la actual.
- «Volver al original» (en el visor, en el menú de la foto) hace que la actual
  sea el original sin borrar el historial: se puede volver a cualquier
  versión. Es la forma de deshacer una edición, igual que en Google Fotos.
- Las fotos editadas llevan en la cuadrícula un icono pequeño de lápiz abajo a
  la derecha.

### Dónde se ve la edición
- Cuadrícula: la miniatura se genera a partir del original y la receta
  actual. La miniatura sin editar se conserva para «Original».
- Visor: la foto se genera en memoria a partir del original y la receta, con
  el lado mayor de 2560 px como mucho (lo mismo que hoy para TIFF y RAW). No
  se guarda en disco.
- Detalles: las dimensiones muestran las de la foto editada y, debajo, las del
  original.
- Exportar la foto editada fuera de la biblioteca va con la spec 008
  (exportar), que leerá esta receta. «Guardar como copia» la deja dentro.

### Formato de la receta
- Una receta por contenido, en `.mnemos/ediciones/<2 primeros>/<huella>.json`,
  con la misma huella BLAKE3 y el mismo reparto en carpetas que las
  miniaturas. Así la edición sigue a la foto aunque se mueva o se renombre
  dentro de la biblioteca, y sobrevive a volver a indexar.
- Si la foto aún no tiene huella, se calcula antes de guardar la primera
  edición.
- Contenido (claves en inglés, como las columnas de la base de datos):

```json
{
  "format": 1,
  "original": { "hash": "9f2c…", "width": 6000, "height": 4000 },
  "current": 2,
  "versions": [
    {
      "id": 1,
      "created_at": 1762190400,
      "geometry": { "rotate": 90, "flip_horizontal": false, "crop": null }
    },
    {
      "id": 2,
      "created_at": 1762194000,
      "geometry": {
        "rotate": 90,
        "flip_horizontal": false,
        "crop": { "x": 0.1, "y": 0.05, "width": 0.8, "height": 0.9 }
      }
    }
  ]
}
```

- `format`: versión del formato. Esta spec define la 1.
- `original`: huella y tamaño del original ya girado según el EXIF. Sirven
  para comprobar que la receta es de ese archivo.
- `current`: id de la versión que se muestra, o `null` si se muestra el
  original.
- Cada versión es una foto completa del estado del editor, no un cambio sobre
  la anterior: cambiar de versión es solo cambiar `current`.
- `geometry.rotate`: 0, 90, 180 o 270, en grados en sentido horario, sobre la
  foto ya girada según el EXIF.
- `geometry.flip_horizontal`: reflejo horizontal, aplicado después del giro.
- `geometry.crop`: rectángulo en fracciones (0 a 1) de la imagen ya girada y
  volteada, o `null` si no hay recorte. Al ser fracciones, la misma receta
  vale para el original, para la vista previa de un RAW y para la miniatura.
- Orden fijo al aplicar: orientación EXIF, giro, volteo, recorte. Las specs
  siguientes añaden `adjustments` (después del recorte) y `annotations` (al
  final) a cada versión, sin cambiar lo anterior.
- Escritura segura: se escribe en `.mnemos/tmp/` y se renombra sobre la
  receta, para que un corte a mitad nunca deje un JSON roto.

### Datos
- La migración 0009 añade a `photos` la columna `edit_version` (id de la
  versión actual, o NULL si se ve el original). Es una caché para la
  cuadrícula y el protocolo `mnemos://`: la receta en disco manda.
- Al indexar, `edit_version` se recalcula leyendo las recetas, así que una
  biblioteca copiada o con la base de datos regenerada recupera sus
  ediciones.
- La miniatura editada se guarda junto a la otra, como
  `miniaturas/v1/<2>/<huella>-e<versión>.jpg`. Las de versiones que ya no son
  la actual se pueden borrar: se regeneran si hacen falta.
- La etiqueta ETag del protocolo incluye la versión, para que el navegador no
  muestre la imagen anterior a la edición.

## Casos límite
- Guardar como copia sin espacio en el disco o con el disco desconectado: la
  copia se escribe en `.mnemos/tmp/` y solo se mueve a `Originales/` y se
  indexa cuando está completa; si falla, no queda ni archivo a medias ni
  registro en la base de datos.
- Guardar como copia de un GIF animado: la copia es un JPEG del primer
  fotograma.
- Disco desconectado al guardar: la receta anterior queda intacta (escritura
  en `tmp/` y renombrado), sale un error y el editor sigue abierto con los
  cambios para reintentar.
- Receta dañada (JSON roto o valores fuera de rango): la foto se muestra sin
  editar, con un aviso en Detalles; el archivo se aparta como
  `<huella>.json.danado` al guardar una edición nueva, nunca se borra.
- Receta de un formato más nuevo (`format` mayor que 1, creada con una versión
  futura de MNEMOS): se aplica lo que se entiende si es seguro, pero el
  editor no deja guardar sobre ella, para no perder lo que no entiende.
- Recorte que, por redondeo, sale de la imagen o mide menos de 1 píxel en la
  miniatura: se ajusta a los bordes y a un mínimo de 1 píxel.
- Dos fotos con el mismo contenido (duplicados exactos): comparten la receta.
  Se avisa en el editor («Esta edición también se verá en otra copia de la
  foto»). La spec 010 de duplicados decidirá qué hacer con ellas.
- Foto desaparecida del disco: no se puede editar (el botón sale
  desactivado), pero su receta se conserva.
- RAW: se edita sobre la vista previa incrustada, como se ve hoy. Cuando haya
  revelado de RAW, la misma receta se aplicará al revelado.
- HEIC y AVIF: sin edición mientras no se puedan decodificar (decisión 004).
- GIF animado: se edita el primer fotograma y se avisa.
- Rutas con caracteres especiales en Windows, macOS y Linux: la receta se
  nombra por la huella (hexadecimal) y la interfaz solo maneja ids.

## Criterios de aceptación
- [ ] Recortar y girar una foto cambia lo que se ve en la cuadrícula y en el
  visor, y el original no se modifica: huella BLAKE3 y fecha de modificación
  iguales antes y después de guardar, cambiar de versión y volver al original
  (prueba automática).
- [ ] Aplicar una receta da el mismo resultado sobre el original y sobre una
  versión reducida, salvo el tamaño (prueba automática con una imagen de
  cuadrantes de colores).
- [ ] Las ocho orientaciones EXIF combinadas con los cuatro giros, el volteo y
  un recorte dan la imagen esperada (prueba automática).
- [ ] Guardar como copia crea un archivo nuevo `nombre (N).ext` en la misma
  carpeta, con la fecha de captura y los datos del original, indexado como
  otra foto, y el original no cambia (prueba automática).
- [ ] Volver al original y luego a la versión 2 muestra exactamente la versión
  2 (prueba automática).
- [ ] Un corte a mitad de guardar deja la receta anterior intacta (prueba
  automática).
- [ ] Una receta dañada o de un formato futuro no rompe la cuadrícula ni el
  visor (prueba automática).
- [ ] Una biblioteca copiada a otra ruta, o con la base de datos borrada y
  vuelta a indexar, conserva sus ediciones (prueba automática).
- [ ] Aplicar la receta para el visor a una foto de 24 MP tarda menos de
  400 ms con giro o volteo, y como mucho el doble que sin editar con un
  recorte grande; mientras tanto el visor enseña la miniatura editada
  ampliada (medición en `docs/rendimiento.md`).
- [ ] Capturas del editor en claro y oscuro aprobadas por Paulo antes de
  cerrar la interfaz.

## Fuera de alcance
- Enderezar con ángulo libre y perspectiva: con los ajustes, en la spec
  siguiente del editor.
- Brillo, contraste, color y filtros: spec siguiente del editor.
- Dibujo, texto y anotaciones: spec propia después de los ajustes.
- Exportar la foto editada fuera de la biblioteca: spec 008.
- Volteo vertical como botón (se consigue con volteo y giro de 180°).
- Editar varias fotos a la vez.
- Escribir la edición en el EXIF o en un XMP del original: nunca, por la regla
  de originales intactos.
- Leer recetas de otros programas (XMP de Lightroom o darktable).

## Tareas (≤1 h cada una)
1. [x] Imagen: tipos de la receta (formato 1) con su validación y lectura
   tolerante, y aplicar la geometría (giro, volteo, recorte) a una imagen ya
   orientada, con pruebas de cuadrantes, orientaciones y redondeos.
2. [x] Imagen: miniatura y vista del visor con receta, reutilizando la
   decodificación reducida de los JPEG. Medición con 24 MP.
3. [x] Núcleo: leer y guardar recetas en `.mnemos/ediciones/` con escritura
   segura, versiones, `current` y volver al original, con pruebas de corte a
   mitad y de recetas dañadas.
4. [x] Migración 0009 con `edit_version` y recálculo al indexar, con prueba
   de migración desde una biblioteca v9 y de biblioteca copiada. Va al
   final de la lista de `db.rs`, detrás de la 0011 (que ocupa el puesto 9).
5. [x] Núcleo: miniaturas editadas y limpieza de las de versiones viejas.
6. [ ] Núcleo: guardar como copia (render a tamaño completo, nombre
   `(N)`, EXIF y datos copiados, escritura en `tmp/` e indexado), con
   pruebas de originales intactos y de fallo a mitad.
7. [ ] Protocolo `mnemos://` y comandos de Tauri: foto y miniatura con la
   receta, guardar como original o como copia, listar versiones y elegir
   versión.
8. [ ] Capturas del diseño del editor (marco de recorte, barra inferior,
   sección Ediciones, icono en la cuadrícula) en claro y oscuro para que Paulo
   las apruebe.
9. [ ] Estado del editor en la interfaz con pruebas: arrastrar esquinas y
   lados, proporciones, giro con el recorte, teclado, cambios sin guardar.
10. [ ] Modo edición en el visor con el marco de recorte y la barra inferior.
11. [ ] Sección Ediciones en Detalles, «Volver al original» e icono de lápiz
    en la cuadrícula.
12. [ ] Prueba de originales intactos de punta a punta; actualizar
    `docs/progreso.md` y `docs/arquitectura.md`.
