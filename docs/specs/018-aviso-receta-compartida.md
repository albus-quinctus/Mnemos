# 018 · Aviso de ediciones compartidas entre duplicados

- Estado: en curso
- Fase: F3 (completa las specs 009 y 010)

## Qué y por qué
Si tengo dos copias idénticas de una foto y edito una, quiero saber antes de
guardar que la otra también cambiará, y poder elegir entre aplicar la edición
a las dos o guardarla como una foto aparte.

## Por qué pasa
Las recetas de edición van por huella BLAKE3 (spec 009,
`.mnemos/ediciones/<ab>/<huella>.json`), no por ruta. Dos fotos con el mismo
contenido (duplicados exactos, spec 010) comparten receta, miniatura editada y
versión actual. Es lo que hace que la edición siga a la foto aunque se mueva,
y no cambia con esta spec: solo se avisa.

## Comportamiento
- **Núcleo:** `Library::identical_copies(id)` devuelve las otras fotos con la
  misma huella que `id` (sin incluirla), con el mismo tipo que las copias de la
  spec 010 (`DuplicateCopy`: id, ruta relativa, si existe en el disco…), por
  orden de llegada. Solo consulta: no escribe en la base de datos ni en el
  disco. Una foto sin huella devuelve una lista vacía.
- **Tauri:** comando `copias_identicas(id)` y su envoltorio `copiasIdenticas`
  en `biblioteca.ts`.
- **Editor:** al entrar, pide las copias idénticas de la foto (en segundo
  plano; si falla, el editor funciona igual, sin aviso). Al pulsar
  **«Guardar como original»** con una o más copias, en lugar de guardar abre un
  diálogo:
  - Título: «Esta foto tiene otra copia idéntica» (o «otras N copias
    idénticas»).
  - Texto: «Las copias idénticas comparten la edición. Si la guardas como
    original, también se verá en:» y la ruta de cada copia (carpeta y nombre),
    hasta tres; si hay más, «y N más». Una copia que no está en el disco se
    marca «(no está en el disco)»: si vuelve, se verá editada.
  - Botones: **«Seguir editando»** (cierra el diálogo, no guarda nada),
    **«Guardar como copia aparte»** (lo mismo que «Guardar como copia»: crea un
    archivo nuevo con la edición y las copias idénticas no cambian) y
    **«Aplicar a las dos»** / **«Aplicar a todas»** (botón principal: guarda
    como original).
  - `Esc` equivale a «Seguir editando».
- **«Guardar como copia»** no avisa: la copia es un archivo nuevo, con otra
  huella, y no afecta a nadie más.
- **Detalles → Ediciones:** cambiar de versión o «Volver al original» también
  afecta a las copias. Si la foto tiene copias idénticas, la sección lleva una
  línea fija debajo del título: «Compartidas con 1 copia idéntica» (o «con N
  copias idénticas»). No pide confirmación: cambiar de versión se puede
  deshacer.

## Casos límite
- Foto sin huella todavía: no hay aviso (sin huella no puede tener receta
  compartida; la huella se calcula al guardar y una copia que tampoco la tenga
  la recibirá al indexar).
- Copias desaparecidas del disco: cuentan y se marcan, porque recuperarían la
  edición al volver.
- Copias archivadas: cuentan como cualquier otra.
- Cientos de copias de la misma foto: el diálogo enseña tres rutas y «y N
  más»; la consulta usa el índice de `content_hash` (sin migración nueva).
- Disco desconectado mientras el editor está abierto: guardar falla como hoy;
  el diálogo no cambia nada por sí mismo.
- Rutas con caracteres especiales en Windows, macOS y Linux: la interfaz solo
  maneja ids; la ruta se muestra como texto.

## Criterios de aceptación
- [x] `identical_copies` devuelve las otras copias de un grupo de 3, sin la
      propia foto y en orden; una foto única o sin huella devuelve una lista
      vacía (prueba automática).
- [x] La consulta no cambia la base de datos ni los originales (prueba
      automática).
- [x] En el editor, «Guardar como original» con una copia idéntica abre el
      diálogo y no guarda hasta elegir; sin copias guarda directamente
      (comprobado en el navegador con la biblioteca simulada).
- [x] «Guardar como copia aparte» desde el diálogo crea la copia y no toca la
      receta compartida; «Aplicar a las dos» guarda como original (comprobado en el
      navegador con la biblioteca simulada).
- [x] Detalles muestra la línea de ediciones compartidas solo cuando hay copias
      (prueba de los textos en `lib/editor/compartida.test.ts`).
- [x] Los originales no se modifican (prueba automática).

## Fuera de alcance
- Separar la receta de una copia (editar una sin que cambie la otra sin crear
  un archivo nuevo): exigiría recetas por ruta y rompería que la edición siga a
  la foto.
- Borrar duplicados: es de la futura papelera.

## Tareas (≤1 h cada una)
1. [x] `Library::identical_copies` en `duplicates.rs`, con pruebas.
2. [x] Comando `copias_identicas` y envoltorio en `biblioteca.ts`.
3. [x] Capturas del diseño del diálogo y de la línea de Detalles, aprobadas
       por Paulo el 12 nov (`capturas/018-*.png`; el editor y el visor son
       oscuros en los dos temas).
4. [x] Diálogo en `Editor.svelte` y línea en Detalles, con pruebas
       (`lib/editor/compartida.ts`).
5. [x] `docs/progreso.md` y estado de la spec.
6. [ ] Probarlo en la app real con dos copias idénticas.
