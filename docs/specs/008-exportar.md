# 008 · Exportar una foto desde el visor

- Estado: en curso
- Fase: F2

## Qué y por qué
Quiero sacar una foto de la biblioteca para mandarla o subirla a otro sitio:
guardarla en una carpeta, tal cual o más pequeña, sin tener que buscar su
archivo dentro de `Originales/`.

Esta spec cubre una sola foto, desde el visor. Exportar varias a la vez
espera a la selección múltiple de la spec 006, y se añadirá como un botón más
de su barra de selección usando el mismo núcleo.

## Comportamiento
- **Botón:** en la cabecera del visor, junto a «Detalles», un botón «Exportar»
  (flecha que sale de una bandeja). Al pulsarlo se abre un menú pequeño con dos
  opciones:
  - **Tamaño original:** copia exacta del archivo, byte a byte, con todos sus
    metadatos (EXIF, ubicación) y su misma extensión. Sirve para cualquier
    formato, también RAW y HEIC.
  - **Tamaño reducido (2048 px):** JPEG con el lado mayor de 2048 píxeles como
    mucho (nunca se amplía), ya girado según la orientación y con calidad 90.
    Se guarda **sin metadatos**: no lleva la ubicación ni la cámara, lo que
    conviene para compartir. El nombre es el del original con extensión `.jpg`.
- Tras elegir la opción se abre el selector de carpetas del sistema
  («Elige dónde guardar la foto»). Si se cancela, no pasa nada. Durante la
  sesión, el selector propone la última carpeta usada.
- **Nombres:** nunca se sobrescribe nada en el destino. Si ya existe
  `foto.jpg`, se guarda como `foto (1).jpg`, `foto (2).jpg`… igual que al
  importar.
- **Aviso:** al terminar, abajo en el visor: «Guardada en <ruta completa>»
  durante unos segundos. Si falla, el motivo en lenguaje claro.
- `Esc` con el menú abierto cierra el menú, no el visor.
- Exportar no cambia nada en la biblioteca: ni la base de datos ni
  `Originales/`. Sin migración.

## Casos límite
- **Destino dentro de la biblioteca:** se rechaza («Elige una carpeta fuera de
  la biblioteca»): la copia aparecería como una foto nueva al indexar.
- **Original que falta en el disco:** «El archivo no está en el disco».
- **Formato que aún no se puede decodificar** (HEIC, AVIF, RAW sin vista
  previa) en tamaño reducido: «Esta foto solo se puede exportar en tamaño
  original».
- **RAW en tamaño reducido:** se usa su vista previa incrustada, como el visor.
- **Destino sin permiso de escritura o disco lleno:** error de E/S con su
  mensaje; el archivo a medias se borra.
- **Disco desconectado a mitad:** igual que el anterior. La copia original se
  verifica con BLAKE3 contra el original leído; si no coincide, se borra.
- **Rutas con espacios, tildes o caracteres especiales** en el nombre o en la
  carpeta de destino (Windows, macOS y Linux).
- **Dos exportaciones con el mismo nombre a la vez:** el nombre se reserva al
  crear el archivo (creación exclusiva), así que no se pisan.

## Criterios de aceptación
- [x] Exportar en tamaño original deja una copia idéntica byte a byte y con la misma fecha de modificación (prueba automática).
- [x] Exportar en tamaño reducido deja un JPEG con el lado mayor ≤ 2048 px, girado y sin EXIF (prueba automática).
- [x] Un nombre repetido en el destino se guarda como `nombre (1).ext` sin tocar el existente (prueba automática).
- [x] Exportar dentro de la biblioteca se rechaza (prueba automática).
- [x] Un formato no decodificable en tamaño reducido da un error claro y no deja archivos (prueba automática).
- [x] El botón y el menú del visor funcionan con ratón y teclado.
- [x] Los originales y la base de datos no se modifican (prueba automática).

## Fuera de alcance
- Exportar varias fotos a la vez (después de la spec 006).
- Exportar la versión editada (spec 009, editor no destructivo): cuando exista,
  el menú ofrecerá «Con ediciones».
- Elegir otros tamaños, formatos (PNG, WebP) o calidad.
- Conservar parte de los metadatos en la reducida (fecha sí, ubicación no).
- Compartir directamente con otras apps o por correo (regla 4: sin nube).

## Tareas (≤1 h cada una)
1. [x] Spec.
2. [x] `mnemos-imaging`: `export_image` y `export_size` (lado mayor 2048 px), reutilizando la decodificación del visor.
3. [x] `mnemos-core`: módulo `export` con `export_photo` (original verificado o reducida), nombres libres con creación exclusiva y destino fuera de la biblioteca. Pruebas.
4. [x] App: comando `exportar` que trabaja fuera del bloqueo de la biblioteca.
5. [x] Visor: botón, menú, selector de carpeta y aviso.
6. [ ] Probarlo en la app real con el HDD (Paulo).
