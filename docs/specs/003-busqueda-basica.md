# 003 · Búsqueda básica

- Estado: aprobada
- Fase: F1

## Qué y por qué
Quiero encontrar una foto escribiendo parte de su nombre, de la carpeta donde está o de su fecha, sin recorrer toda la cuadrícula. Es lo último que falta de F1 para que la biblioteca sea útil con 10 000 fotos o más.

## Comportamiento
- **Dónde:** se usa el campo "Buscar en tus fotos" de la barra superior, que hoy está desactivado.
- **Al escribir:** la cuadrícula se filtra al momento. Se espera 200 ms desde la última tecla antes de buscar. Las fotos siguen agrupadas por día, con la misma cuadrícula justificada y la misma barra de meses, en la que solo aparecen los meses con resultados.
- **Encima de la cuadrícula** se lee "N fotos para «texto»". Con `Esc` o con la cruz del campo se vuelve a la biblioteca entera, en el punto donde estaba.
- **Palabras:** se buscan en el nombre del archivo y en las carpetas de su ruta. No distinguen mayúsculas ni acentos, y cada palabra vale como inicio de palabra: `cumple` encuentra `Cumpleaños de Ana/IMG_0001.jpg` y `img_00` encuentra `IMG_0012.jpg`. Si se escriben varias palabras, la foto debe tener todas.
- **Fechas:** algunas formas se reconocen como fecha y filtran por el día de la foto (la fecha de captura o, si no la hay, la de modificación, en hora local):
  - un año: `2024`;
  - mes y año: `marzo 2024`, `mar 2024`, `2024-03` o `03/2024`;
  - un día: `26/10/2026` o `2026-10-26`;
  - un mes o un día sin año (`marzo`, `26/10`): se toma el más reciente que tenga fotos. Si hoy es octubre de 2026 y hay fotos de marzo de 2024 y de 2026, `marzo` es marzo de 2026. Si hay palabras, es el más reciente con resultados: `playa marzo` da el último marzo con fotos de la playa.
  Una fecha se combina con las palabras: `playa 2024` son las fotos de 2024 cuya ruta contiene `playa`. Si un número no encaja como fecha, se busca como palabra.
- **Visor:** al abrir una foto desde los resultados, las flechas y la tira recorren solo los resultados.
- **Sin resultados:** "No hay fotos para «texto»", con una pista: se busca por nombre, carpeta o fecha.
- La búsqueda nunca toca los archivos: solo consulta la base de datos.

## Casos límite
- Texto con comillas, asteriscos, guiones, paréntesis u operadores de FTS5 (`AND`, `NEAR`): se tratan como texto normal y nunca producen un error de sintaxis.
- Texto vacío o solo espacios: se muestra la biblioteca entera.
- Fechas imposibles (`31/02/2024`, `2024-13`): se buscan como palabras y no dan error.
- Una foto movida o renombrada fuera de la aplicación: al volver a indexar se encuentra por su ruta nueva. El registro viejo sigue como foto desaparecida, igual que en la cuadrícula. Qué hacer con las desaparecidas sigue fuera de alcance.
- Fotos desaparecidas del disco: siguen apareciendo en los resultados, como en la cuadrícula.
- Rutas con caracteres especiales en Windows, macOS y Linux: la interfaz solo maneja ids.
- 10 000 fotos o más: los resultados de cualquier búsqueda llegan en menos de 100 ms.
- Disco desconectado mientras se busca: error claro, como en la cuadrícula.

## Criterios de aceptación
- [x] `cumple` encuentra `Cumpleaños/IMG_0001.jpg`, y `CUMPLEANOS` también (prueba automática).
- [x] `playa 2024` solo devuelve fotos de 2024 cuya ruta contiene `playa` (prueba automática).
- [x] `marzo 2024`, `2024-03` y `03/2024` dan el mismo resultado (prueba automática).
- [x] `marzo` sin año da el marzo más reciente con fotos (prueba automática).
- [x] Ningún texto de entrada provoca un error de FTS5. Se prueba con una lista de textos raros y con entradas al azar (prueba automática).
- [x] Al mover un archivo a otra carpeta y volver a indexar, la búsqueda por el nombre de la carpeta nueva lo encuentra (prueba automática).
- [x] Una búsqueda en 10 000 fotos sintéticas responde en menos de 100 ms (en `docs/rendimiento.md`).
- [ ] Las flechas del visor recorren solo los resultados.
- [x] Los originales no se modifican (prueba automática).

## Fuera de alcance
- EXIF (cámara, lugar, objetivo) y la fecha de captura leída del EXIF: F2.
- Etiquetas, álbumes y favoritos en la búsqueda: F2. La columna `tags` del índice ya existe y se queda vacía.
- Búsqueda por contenido de la imagen (caras, objetos): no está previsto.
- Búsquedas guardadas, historial y sugerencias mientras se escribe.
- Otros órdenes que no sean del más reciente al más antiguo.
- Operadores (`-palabra`, `OR`, comillas para frases exactas).

## Tareas (≤1 h cada una)
1. [x] Intérprete de la consulta en `mnemos-core`: separa palabras y fechas y escapa las palabras para FTS5. Solo lógica, con pruebas, incluidas entradas al azar.
2. [x] Comprobar que `photo_search` se mantiene al indexar, actualizar e importar, y corregirlo si no. Si hace falta, una migración que reconstruya el índice.
3. [x] Consulta paginada de resultados y recuento por mes en `mnemos-core`, con el mismo cursor que la cuadrícula, y su medición con 10 000 fotos.
4. [ ] Comandos de Tauri `buscar_meses` y `buscar_fotos`.
5. [ ] Capturas del diseño del campo y de los resultados para que Paulo las apruebe.
6. [ ] Campo de búsqueda activo, con espera de 200 ms y `Esc`. Cuadrícula y barra de meses con resultados, recuento y mensaje sin resultados.
7. [ ] Visor dentro de los resultados.
