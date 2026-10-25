# 001 · Importar e indexar fotos

- Estado: en curso
- Fase: F1

## Qué y por qué
Quiero meter mis fotos en la biblioteca y que MNEMOS sepa cuáles tiene, sin arriesgar ni los archivos de origen ni los originales que ya están en el disco.

## Comportamiento

### Indexar
- Recorre `Originales/` y registra en la base de datos cada imagen reconocida: ruta relativa, tamaño, fecha de modificación, huella BLAKE3 y dimensiones.
- Ignora archivos y carpetas ocultos (que empiezan por `.`) y los que no son imágenes.
- Si una foto ya está registrada con el mismo tamaño y fecha, no la vuelve a leer.
- Si cambió el tamaño o la fecha, actualiza su registro (huella y dimensiones incluidas).
- Si una foto registrada ya no está en el disco, la cuenta como desaparecida **pero no borra su registro**: puede ser un disco a medio copiar o un error del usuario. Qué hacer con ellas se decide en otra spec.
- Un archivo que no se puede leer no detiene el proceso: se anota como error y se sigue.

### Importar
- Copia las imágenes de una carpeta de origen (con subcarpetas) a `Originales/AAAA/MM/`, según la fecha de modificación del archivo. La fecha EXIF llega en F2.
- **Nunca modifica, mueve ni borra el origen.**
- Cada copia se escribe primero en `.mnemos/tmp/`, se comprueba que su huella coincide con la del origen y solo entonces se mueve a su sitio. Si no coincide, se descarta y se anota el error.
- Conserva la fecha de modificación del origen en la copia.
- Si ya hay en la biblioteca una foto con la misma huella, no se copia: cuenta como duplicada. Lo mismo dentro de una misma importación.
- Si ya existe un archivo con el mismo nombre en el destino, añade un sufijo (`foto (1).jpg`) en vez de sobrescribir.
- Rechaza un origen que esté dentro de la propia biblioteca.
- Cada foto copiada queda indexada al terminar.

## Casos límite
- Disco desconectado a mitad de operación: la copia a medias queda en `.mnemos/tmp/`, nunca en `Originales/`.
- Archivos corruptos o con cabecera ilegible: se indexan sin dimensiones.
- Nombres con caracteres especiales y rutas de Windows, macOS y Linux: se guardan con `RelPath`.
- Archivos con nombres no UTF-8: se anotan como error y se saltan.

## Criterios de aceptación
- [x] Indexar dos veces seguidas no cambia nada la segunda vez.
- [x] Un archivo modificado se actualiza; uno borrado se cuenta como desaparecido y su registro sigue ahí.
- [x] Importar no cambia ni el contenido ni la fecha de los archivos de origen (prueba automática).
- [x] Importar el mismo contenido dos veces no crea copias.
- [x] Un nombre repetido en el destino no sobrescribe nada.
- [x] Las órdenes `mnemos index` y `mnemos import` muestran un resumen.

## Fuera de alcance
- Miniaturas, cuadrícula y visor (spec 002).
- Lectura de EXIF y fecha de captura (F2).
- Barra de progreso en la aplicación de escritorio (spec 002).
- Eliminar registros de fotos desaparecidas.

## Tareas
1. [x] Dimensiones de imagen en `mnemos-imaging`.
2. [x] Indexado de `Originales/` en `mnemos-core`.
3. [x] Importación con copia verificada.
4. [x] Órdenes `index` e `import` en la CLI.
