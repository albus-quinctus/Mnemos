# 003 · Estilo visual de la interfaz

- Fecha: 2026-10-25
- Estado: aceptada (2026-10-25)

## Contexto
La spec 002 introduce la cuadrícula de miniaturas y el visor, y necesita un estilo visual definido. Se compararon cuatro direcciones en un lienzo de diseño: Galería, Cuarto oscuro, Archivo y Hoja de contactos.

## Decisión
- **Cuadrícula y navegación: dirección "Galería".** Fondo neutro, menú lateral (Fotos, Explorar, Álbumes, Etiquetas, Favoritos, Papelera), barra de búsqueda arriba y cuadrícula justificada agrupada por fecha. Tipografía Inter y acento azul (`#1a5fd0`).
- **Visor: dirección "Cuarto oscuro".** Fondo casi negro, la foto ocupa el centro, tira de miniaturas abajo y panel lateral de detalles (EXIF, ruta relativa, etiquetas y versiones). La lista de versiones deja claro que el original está intacto.
- Colores definidos como variables CSS (tokens) con un juego claro y otro oscuro, para que ambos modos salgan de la misma base y se respete la preferencia del sistema.
- Iconos de trazo en SVG en línea, sin fuentes de iconos externas, y fuentes empaquetadas con la aplicación (sin Google Fonts en tiempo de ejecución) para funcionar sin conexión.
- Accesibilidad: objetivos de clic de al menos 44 px y contraste de texto de 4,5:1.

## Alternativas consideradas
- **Archivo** (editorial, serif): elegante para álbumes, pero menos familiar para quien viene de Google Fotos. Puede inspirar la vista de álbumes más adelante.
- **Hoja de contactos** (densa, monoespaciada, árbol de carpetas): útil para usuarios avanzados. Podría ofrecerse como modo de vista alternativo.

## Consecuencias
- La spec 002 implementa los tokens de color, la cuadrícula justificada y el visor oscuro.
- Las fuentes Inter (y una monoespaciada para los metadatos) se incluyen en el paquete de la aplicación.
