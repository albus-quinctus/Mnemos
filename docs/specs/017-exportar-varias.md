# 017 · Exportar varias fotos a la vez

- Estado: borrador
- Fase: F2

## Qué y por qué
Quiero sacar de golpe todas las fotos de un viaje o de un día para pasarlas a
otra persona o a un pendrive, sin abrirlas una a una en el visor. Es la
exportación de la spec 008 llevada a la barra de selección de la spec 006, con
el mismo núcleo.

## Comportamiento
- **Botón:** en la barra de selección, tras «Archivar», un botón «Exportar»
  con el mismo icono que el visor. Abre el mismo menú de dos opciones:
  - **Tamaño original:** copia exacta de cada archivo, con sus metadatos.
  - **Tamaño reducido (2048 px):** JPEG sin metadatos, como en la spec 008.
- Tras elegir, el selector de carpetas del sistema: «Elige dónde guardar las
  N fotos». Si se cancela, no pasa nada. Propone la última carpeta usada en la
  sesión, la misma que el visor.
- **Qué se exporta de cada foto:** exactamente lo mismo que exportaría el
  visor. Si la foto tiene una versión editada, se exporta esa (hilo «Exportar
  la foto editada», que añade esa regla a la spec 008); el lote solo llama a
  la exportación de una foto para cada una.
- **Orden y nombres:** en el orden de la cuadrícula. Nunca se sobrescribe nada:
  si dos fotos se llaman `IMG_0001.jpg` (de meses distintos) o el nombre ya
  existe en el destino, la siguiente se guarda como `IMG_0001 (1).jpg`, igual
  que en la spec 008. Todas las fotos van a la misma carpeta, sin subcarpetas.
- **Progreso:** una tarjeta abajo a la derecha, como la de importar:
  «Exportando 124 de 340…», una barra, la carpeta de destino y el botón
  «Cancelar». Mientras dura, el botón «Exportar» de la barra está
  desactivado; la selección y el resto de la app se pueden seguir usando (el
  lote trabaja con los ids del momento de empezar).
- **Cancelar:** termina la foto en curso y para. Lo ya exportado se queda en
  el destino; nada a medias. La tarjeta dice «Exportación cancelada · 12 de
  340 guardadas en <carpeta>».
- **Resumen al terminar,** en la misma tarjeta, que se queda hasta pulsar
  «Cerrar»:
  - Todo bien: «340 fotos guardadas · En <carpeta>». Con una sola foto,
    «Foto guardada · En <ruta completa>», como el visor.
  - Con fotos saltadas: «335 de 340 guardadas · 5 fotos no se pudieron
    exportar» y el botón «Ver», que despliega dentro de la tarjeta la lista
    con el nombre de cada foto y el motivo en lenguaje claro.
- La selección se mantiene al terminar, para poder repetir con otro tamaño.
- Exportar no cambia nada en la biblioteca: ni la base de datos ni
  `Originales/`. Sin migración.

## Errores: cuándo se salta una foto y cuándo se para todo
- **Se salta la foto y se sigue** (sale en «Ver»): el original no está en el
  disco; formato que no se puede reducir (HEIC, AVIF, RAW sin vista previa:
  «Solo se puede exportar en tamaño original»); la copia no coincide con el
  original tras verificarla con BLAKE3; una edición que no se puede aplicar
  (el error que añada el hilo «Exportar la foto editada»).
- **Se para todo** (la tarjeta dice «Se detuvo la exportación», el motivo y lo ya guardado): no se puede escribir en
  el destino (sin permiso, disco lleno, pendrive quitado) o el disco de la
  biblioteca se ha desconectado. Seguir solo llenaría la lista con el mismo
  fallo.
- **Destino dentro de la biblioteca:** se rechaza antes de empezar, como en la
  spec 008.

## Casos límite
- Selecciones grandes (10 000 fotos con `Ctrl` + `A`): una foto detrás de otra
  en un hilo aparte, sin bloquear la biblioteca ni la interfaz; el progreso se
  actualiza como mucho unas 10 veces por segundo.
- Fotos desaparecidas en la selección: se saltan con «El archivo no está en el
  disco».
- Exportar mientras otra exportación está en marcha (desde el visor o la
  barra): el visor sigue funcionando por su cuenta; la barra no deja empezar
  un segundo lote.
- Cerrar la app a mitad: el archivo en curso se borra al fallar su escritura
  si da tiempo; lo ya exportado se queda.
- Rutas con espacios, tildes o caracteres especiales en Windows, macOS y Linux
  (lo cubre la spec 008).

## Criterios de aceptación
- [ ] Exportar un lote deja una copia de cada foto en el destino, con los nombres repetidos como `nombre (N).ext` y sin tocar los archivos que ya había (prueba automática).
- [ ] Una foto que falta o que no se puede reducir se salta, sale en el resumen con su motivo y el resto se exporta (prueba automática).
- [ ] Un error al escribir en el destino detiene el lote y lo ya exportado se queda entero (prueba automática).
- [ ] Cancelar para tras la foto en curso sin dejar archivos a medias (prueba automática).
- [ ] Una foto con versión editada se exporta igual que desde el visor (prueba automática, cuando exista la exportación de editadas).
- [ ] El botón, el menú, el progreso, «Cancelar» y «Ver» funcionan con ratón y teclado.
- [ ] Capturas del botón, el progreso y el resumen aprobadas por Paulo.
- [ ] Los originales y la base de datos no se modifican (prueba automática).

## Fuera de alcance
- Comprimir en un `.zip`.
- Subcarpetas por fecha o por día en el destino.
- Abrir la carpeta de destino en el gestor de archivos al terminar.
- Comprobar el espacio libre antes de empezar (el error de disco lleno para el lote).
- Otros tamaños, formatos o calidad (fuera de alcance también en la spec 008).
- Barra de selección con solo iconos cuando se llene con álbumes, etiquetas y papelera.

## Tareas (≤1 h cada una)
1. [ ] Spec y capturas del diseño.
2. [ ] `mnemos-core`: `export_many` en un módulo aparte, con progreso, cancelación y la regla de saltar o parar. Pruebas. Llama a la exportación de una foto de la spec 008 (con la versión editada, cuando esté).
3. [ ] App: comandos `exportar_varias` y `cancelar_exportacion` con el evento `exportacion` (en curso, terminada, cancelada, detenida), en un hilo aparte como la importación.
4. [ ] Barra de selección: botón y menú, selector de carpeta, progreso con «Cancelar», resumen y «Ver». Pruebas de la interfaz.
5. [ ] Probarlo en la app real con el HDD (Paulo).
