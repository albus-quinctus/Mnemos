# 010 · Duplicados exactos

- Estado: en curso
- Fase: F4 (adelantada a F2)

## Qué y por qué
Quiero saber qué fotos tengo repetidas en la biblioteca, byte a byte, para decidir yo qué hacer con ellas. MNEMOS solo las encuentra y las enseña: no borra, no mueve y no marca nada.

## Cómo aparecen duplicados
`mnemos import` ya no copia una foto cuya huella BLAKE3 está en la biblioteca, así que los duplicados exactos llegan por otras vías:
- archivos copiados a mano dentro de `Originales/` y registrados con `mnemos index`;
- bibliotecas creadas sobre una carpeta que ya tenía fotos repetidas;
- una foto movida o renombrada fuera de la aplicación: la ruta vieja sigue como foto desaparecida y la nueva tiene la misma huella.

## Comportamiento
- **Qué es un duplicado:** dos o más registros de `photos` con el mismo `content_hash`. Mismo contenido exacto, sin comparar píxeles ni metadatos. Las fotos sin huella (`content_hash` nulo) no cuentan.
- **Resumen:** número de grupos, número de copias de más (en cada grupo, todas menos una) y espacio que ocupan esas copias de más.
- **Grupos:** cada grupo es una huella con sus copias. Se listan del grupo con la foto más reciente al más antiguo (por la misma fecha que la cuadrícula: captura o, si falta, modificación), y se paginan por cursor (fecha e huella) para que 100 000 fotos cuesten lo mismo.
- **Copias de un grupo:** id, ruta relativa, tamaño, fecha, si es favorita, si está archivada y si el archivo sigue en el disco. Se ordenan por la fecha en que entraron en la biblioteca (`imported_at`) y luego por id: la primera es la que llegó antes. Ese orden no es una recomendación de cuál conservar.
- **Archivadas y favoritas:** cuentan como cualquier otra foto. Se indica para que el usuario lo vea al decidir.
- **Fotos desaparecidas:** se muestran dentro de su grupo marcadas como desaparecidas, porque explican muchos duplicados (fotos movidas a mano). Un grupo con una copia en el disco y otra desaparecida sigue siendo un grupo.
- **CLI:** `mnemos duplicados <biblioteca>` imprime el resumen y cada grupo con sus rutas. Sale con código 0 haya o no duplicados.
- **Interfaz:** una vista "Duplicados" con el resumen arriba y un grupo por fila: la miniatura (es la misma para todas las copias, porque va por huella) y la lista de copias con su carpeta, nombre, tamaño y avisos (favorita, archivada, desaparecida). Al pulsar una copia se abre en el visor. Sin selección ni botones de borrar.
- La detección solo consulta la base de datos (y comprueba si cada archivo existe). Nunca escribe en los originales ni en la base de datos.

## Casos límite
- Biblioteca sin duplicados: resumen a cero y el mensaje "No hay fotos repetidas".
- Una foto modificada después de indexar: su huella en la base de datos es la vieja hasta el siguiente `mnemos index`. Se muestra lo que dice la base de datos; la vista sugiere volver a indexar si algo no cuadra.
- Mismo archivo con distinta extensión o nombre: es un duplicado (mismo contenido).
- Misma foto recomprimida, redimensionada o con EXIF distinto: **no** es un duplicado exacto (fuera de alcance).
- Grupos grandes (cientos de copias de la misma imagen): se devuelven todas las copias del grupo; la página limita el número de grupos.
- 10 000 fotos o más: el resumen y la primera página tardan menos de 100 ms (hay índice sobre `content_hash` desde la migración 0001; no hace falta migración nueva).
- Disco desconectado: todas las copias salen como desaparecidas y la vista lo dice con el mismo error que la cuadrícula.
- Rutas con caracteres especiales en Windows, macOS y Linux: la interfaz solo maneja ids; la ruta se muestra como texto.

## Criterios de aceptación
- [x] Dos archivos idénticos en carpetas distintas de `Originales/` forman un grupo tras `mnemos index`; uno distinto no aparece (prueba automática).
- [x] El resumen cuenta bien grupos, copias de más y bytes con grupos de 2 y de 3 copias (prueba automática).
- [x] Las fotos sin huella no forman grupo (prueba automática).
- [x] La paginación por cursor devuelve todos los grupos una sola vez, en orden (prueba automática).
- [x] Una copia borrada del disco sale marcada como desaparecida (prueba automática).
- [x] Ni la consulta ni la CLI modifican originales ni la base de datos (prueba automática).
- [x] Resumen y primera página en menos de 100 ms con 10 000 fotos sintéticas.
- [ ] Vista "Duplicados" en la aplicación, aprobada por Paulo con capturas.

## Fuera de alcance
- Borrar, mover, fusionar o "quedarse con una" copia. Llegará con la papelera, que necesita una excepción a la regla 1 de `CLAUDE.md`.
- Fotos parecidas (recomprimidas, recortadas, ráfagas): necesitan huella perceptual, otra spec.
- Comparar con fotos fuera de la biblioteca (eso ya lo hace la importación).
- Volver a calcular huellas: lo hace `mnemos index`.

## Tareas (≤1 h cada una)
1. [x] Consulta de resumen y grupos paginados en `mnemos-core`, con pruebas y medición con 10 000 fotos.
2. [x] `mnemos duplicados <biblioteca>` en la CLI.
3. [x] Comandos de Tauri `duplicados_resumen` y `duplicados_grupos`.
4. [ ] Capturas del diseño de la vista para que Paulo las apruebe. Espera a que la spec 006 fije la navegación lateral (Fotos, Favoritos, Archivo) para añadir "Duplicados" ahí sin chocar.
5. [ ] Vista "Duplicados" con resumen, grupos y apertura en el visor.
