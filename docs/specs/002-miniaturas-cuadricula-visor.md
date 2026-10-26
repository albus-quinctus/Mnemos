# 002 · Miniaturas, cuadrícula y visor

- Estado: en curso
- Fase: F1

## Qué y por qué
Quiero ver mis fotos en la aplicación: recorrer toda la biblioteca en una cuadrícula fluida, abrir cualquier foto a pantalla completa e importar carpetas nuevas sin usar la terminal. Todo debe ir ligero desde un disco duro externo con 10 000 fotos o más, y aceptar fotos de cualquier dispositivo: móviles, cámaras y RAW.

## Enfoques considerados
- **Elegido: miniaturas pequeñas en disco y visor sin caché intermedia.** Solo se guarda una miniatura por foto. El visor muestra la miniatura ampliada al instante, carga encima el original o su vista previa y precarga en memoria la foto anterior y la siguiente. Ocupa unos 250 MB por cada 10 000 fotos.
- Descartado: guardar además una vista de 2048 px por foto. El visor siempre sería instantáneo, pero ocuparía unos 3 GB por cada 10 000 fotos y alargaría la primera indexación. Si el visor se queda corto, se puede añadir más adelante como caché en disco con un límite de tamaño.
- Descartado: revelar los RAW. Es lento y pesado; encaja con la edición de F3. Para ver los RAW basta con la vista previa en JPEG que trae el propio archivo.

## Comportamiento

### Formatos
- El índice reconoce, además de los formatos actuales (JPEG, PNG, WebP, GIF, BMP, TIFF, HEIC, HEIF y AVIF), los RAW más comunes: `dng`, `cr2`, `cr3`, `nef`, `nrw`, `arw`, `raf`, `orf`, `rw2`, `pef` y `srw`.
- **JPEG, PNG, WebP, GIF, BMP y TIFF** se decodifican con Rust puro, sin librerías en C.
- **RAW y DNG**: la miniatura y el visor usan la vista previa en JPEG incrustada en el archivo. No se revela el RAW. Si un RAW no trae vista previa, se muestra un marcador "Sin vista previa" con el nombre del archivo.
- **HEIC, HEIF y AVIF** necesitan `libheif`, una librería en C. Antes de añadirla hay que registrar la decisión 004, que cubre la licencia, el empaquetado portátil en los tres sistemas y las alternativas. Mientras tanto, estas fotos se muestran con el marcador "Sin vista previa".
- La librería para leer las vistas previas de los RAW se elige en esa misma decisión 004. Debe ser de Rust puro o, como mínimo, no depender de código en C.

### Orientación
- Al indexar se lee la etiqueta de orientación EXIF. Es la única lectura de EXIF de esta spec; el resto llega en F2.
- La miniatura se guarda ya girada. Las dimensiones que usa la cuadrícula son las de la foto ya girada, para que una foto vertical de móvil no salga tumbada.

### Miniaturas
- Cada miniatura es un JPEG de 360 px de alto con el ancho proporcional, con un tope de 1080 px de ancho para las panorámicas, y calidad 80. Ocupa unos 25 KB.
- Se guardan en `.mnemos/miniaturas/v1/<2 primeros caracteres de la huella>/<huella>.jpg`. Al ir por la huella BLAKE3, dos copias iguales comparten miniatura, y renombrar o mover una foto no obliga a regenerarla. La carpeta `v1` permite regenerarlas todas si cambia el tamaño o la calidad.
- Para crear una miniatura nunca se decodifica la foto completa si se puede evitar: de un JPEG se decodifica una versión reducida y de un RAW solo se extrae su vista previa.
- Se generan en segundo plano después de importar o indexar, y también cuando falta alguna al abrir la biblioteca. Las fotos que hay en pantalla van primero.
- Se trabaja con pocos hilos, 2 por defecto, porque en un HDD leer muchos archivos a la vez es más lento que leerlos en orden.
- Una miniatura se escribe primero en `.mnemos/tmp/` y luego se mueve a su sitio, así que un corte nunca deja una miniatura a medias.
- Si una foto no se puede decodificar, se anota el error y se muestra el marcador. No se reintenta en cada arranque, solo cuando cambia el archivo.

### Servir imágenes a la interfaz
- La interfaz pide las imágenes con un protocolo propio de Tauri, por ejemplo `mnemos://miniatura/<id>` y `mnemos://foto/<id>`, usando el **id de la foto, nunca una ruta**. El núcleo traduce el id a su ruta relativa. Así la interfaz no puede pedir ningún archivo fuera de la biblioteca.
- `mnemos://foto/<id>` entrega el original si el motor web lo puede mostrar (JPEG, PNG, WebP, GIF y BMP). En los demás casos, como TIFF en Windows, RAW o HEIC, entrega un JPEG generado en memoria a partir de la vista previa o del original, con el lado mayor ajustado a la pantalla. Ese JPEG no se guarda en disco.
- La CSP se actualiza para permitir solo ese protocolo en `img-src`.

### Cuadrícula (estilo Galería, decisión 003)
- Cuadrícula justificada: filas de altura parecida, cada foto con su proporción y sin recortes.
- Agrupada por mes con encabezados ("Octubre de 2026"), de lo más reciente a lo más antiguo. Hasta que llegue la fecha EXIF en F2 se usa la fecha de modificación.
- Virtualizada: solo están en el DOM las filas visibles y un margen. Las fotos se piden al núcleo por páginas, nunca todas a la vez.
- Mientras una miniatura no está lista se ve un hueco con el color de fondo y las dimensiones correctas, así que la cuadrícula no salta al cargar.
- Menú lateral, barra superior y tokens de color claro y oscuro según la decisión 003. Las entradas del menú que aún no existen (Explorar, Álbumes, Etiquetas, Favoritos y Papelera) se ven desactivadas.
- La búsqueda queda fuera de esta spec.

### Visor (estilo Cuarto oscuro, decisión 003)
- Se abre con clic o con Intro sobre una foto y se cierra con Esc.
- Al abrir, muestra al instante la miniatura ampliada y la sustituye por la imagen completa en cuanto termina de cargar.
- Flechas izquierda y derecha para pasar de foto. La anterior y la siguiente se precargan en memoria. La caché del visor guarda como mucho 5 imágenes completas.
- Tira de miniaturas abajo y panel lateral de detalles, que se abre y se cierra con la tecla `i`. El panel muestra la ruta relativa, las dimensiones, el tamaño del archivo, la fecha de modificación y el formato. El EXIF, las etiquetas y las versiones llegan en F2 y F3.
- Una foto desaparecida del disco se muestra con su miniatura, si existe, y el aviso "El archivo no está en el disco".

### Abrir biblioteca e importar desde la aplicación
- Al arrancar, la aplicación abre la última biblioteca usada. Si no existe, ofrece abrir una biblioteca o crear una nueva, eligiendo la carpeta con el diálogo del sistema (plugin `dialog`).
- La ruta de la última biblioteca se guarda en la configuración de la aplicación, no en la base de datos, porque es absoluta y depende de cada equipo. Si la aplicación arranca desde el disco externo, se ofrece la biblioteca que está junto al ejecutable.
- Botón "Importar": elige una carpeta de origen con el diálogo y ejecuta la importación de la spec 001 en segundo plano.
- Mientras importa se ve el progreso: fotos revisadas sobre el total, copiadas, duplicadas y errores. La aplicación se puede seguir usando.
- La importación se puede cancelar. Lo ya copiado se queda y lo que estaba a medias se descarta de `.mnemos/tmp/`, como en la spec 001.
- Al terminar se muestra el resumen, las fotos nuevas aparecen en la cuadrícula y sus miniaturas se generan.

## Casos límite
- Disco desconectado a mitad de importación o de generación de miniaturas: la operación se detiene con un error claro, sin miniaturas a medias. Al volver a conectarlo, se reanuda lo que falte.
- Foto vertical de móvil con orientación EXIF: la miniatura y el visor la muestran derecha.
- RAW sin vista previa, archivo corrupto o formato todavía sin soporte: marcador "Sin vista previa" y error anotado una sola vez.
- Panorámica muy ancha o foto muy estrecha: el tope de 1080 px y la cuadrícula justificada evitan filas absurdas.
- 10 000 fotos o más: el desplazamiento sigue fluido y la memoria no crece con el tamaño de la biblioteca.
- Dos fotos idénticas en rutas distintas: comparten miniatura.
- Rutas con caracteres especiales en Windows, macOS y Linux: la interfaz solo maneja ids, nunca rutas.
- Petición al protocolo con un id que no existe: respuesta 404, sin tocar el sistema de archivos.

## Criterios de aceptación
- [ ] Indexar una carpeta de prueba con JPEG, PNG, TIFF y DNG genera una miniatura para cada uno en `.mnemos/miniaturas/v1/`, de 360 px de alto.
- [ ] Una foto de prueba con orientación EXIF 6 genera una miniatura vertical.
- [ ] Generar las miniaturas dos veces seguidas no vuelve a escribir ninguna.
- [ ] Un archivo corrupto no detiene la generación y queda anotado como error.
- [ ] El protocolo `mnemos://` rechaza ids inexistentes y nunca sirve archivos fuera de la biblioteca (prueba automática).
- [ ] La cuadrícula mantiene 60 fps al desplazarse por 10 000 fotos en un equipo de gama media. Se mide con una biblioteca sintética generada en las pruebas, nunca con una biblioteca real.
- [ ] El visor muestra una imagen en menos de 100 ms al abrir una foto y al pasar a la siguiente.
- [ ] Importar desde la aplicación muestra el progreso, se puede cancelar y no deja nada en `.mnemos/tmp/`.
- [ ] Los originales no se modifican: ni el contenido ni la fecha cambian al generar miniaturas ni al verlos (prueba automática).

## Fuera de alcance
- Revelado de RAW: F3.
- Miniaturas de HEIC, HEIF y AVIF hasta que se acepte la decisión 004.
- Lectura de EXIF aparte de la orientación, y fecha de captura: F2.
- Búsqueda, álbumes, etiquetas, favoritos y papelera.
- Vídeo.
- Caché en disco de vistas grandes para el visor.
- Qué hacer con las fotos desaparecidas.

## Tareas (≤1 h cada una)
1. [x] Decisión 004: librerías para las vistas previas RAW y para HEIC/AVIF, con licencia y empaquetado.
2. [x] Extensiones RAW en `mnemos-imaging` y lectura de la orientación EXIF al indexar (migración 0002).
3. [x] Miniaturas de JPEG, PNG, WebP, GIF, BMP y TIFF con orientación en `mnemos-imaging`.
4. [x] Miniaturas de RAW a partir de la vista previa incrustada.
5. [x] Cola de miniaturas en segundo plano en `mnemos-core`, con prioridad, hilos limitados y escritura vía `.mnemos/tmp/`.
6. [x] Protocolo `mnemos://` en la aplicación, con conversión en memoria para los formatos que el motor web no muestra, y la nueva CSP.
7. [ ] Consulta paginada de fotos agrupadas por mes en `mnemos-core` y su comando de Tauri.
8. [ ] Tokens de color, fuentes empaquetadas y estructura de la Galería: menú lateral y barra superior.
9. [ ] Cuadrícula justificada y virtualizada.
10. [ ] Visor: apertura con la miniatura, carga del original, flechas, precarga y tira de miniaturas.
11. [ ] Panel de detalles del visor.
12. [ ] Abrir o crear una biblioteca con el diálogo y recordar la última.
13. [ ] Importar desde la aplicación con progreso y cancelación.
14. [ ] Miniaturas de HEIC y AVIF, si la decisión 004 se acepta.
15. [ ] Prueba con 10 000 fotos sintéticas y medición de tiempos.
