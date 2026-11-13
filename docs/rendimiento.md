# Rendimiento

Mediciones de la spec 002 (tarea 15) y de la búsqueda de la spec 003 (tarea 3) con bibliotecas sintéticas. Nunca se usa una biblioteca real.

## Cómo repetirlas
- Núcleo: `cargo test --release -p mnemos-core --test rendimiento -- --ignored --nocapture`. `MNEMOS_FOTOS=50000` cambia el tamaño.
- Disposición de la cuadrícula: `pnpm test`. Las pruebas de 10 000 fotos están en `apps/desktop/src/lib/cuadricula/disposicion.test.ts` y corren siempre, con límites holgados.
- Cuadrícula y visor en el navegador: se sirve la interfaz con `pnpm build && pnpm preview` y Playwright simula la API de Tauri con 10 000 fotos. Mide los fotogramas durante 3 s de desplazamiento continuo y el tiempo hasta que el visor pinta la foto.

## Resultados del 2026-10-26
Equipo: contenedor Linux con 4 núcleos Xeon a 2,1 GHz, disco SSD y Chromium sin pantalla. Las fotos son JPEG de 450 a 1024 px repartidas en tres años.

### Núcleo, 10 000 fotos
| Paso | Tiempo |
|---|---|
| Indexar | 0,52 s |
| Volver a indexar sin cambios | 30 ms |
| Generar las miniaturas (2 hilos) | 100 s, unos 10 ms por foto |
| Meses de la barra lateral | 7 ms |
| Recorrer toda la cuadrícula en páginas de 500 | 20 ms en total; la página más lenta, 1,6 ms |
| Detalles de una foto | 0,07 ms |
| Espacio de las miniaturas | 68 MB |

### Búsqueda en el núcleo, 10 000 fotos
Cada tiempo es lo que tarda la interfaz en tener los meses con resultados y la primera página de 500. El objetivo de la spec 003 es menos de 100 ms.

| Búsqueda | Fotos encontradas | Tiempo |
|---|---|---|
| `img` (todas) | 10 000 | 25 ms |
| `originales` (todas) | 10 000 | 24 ms |
| `img 2024` | 2762 | 15 ms |
| `marzo` (sin año, el más reciente) | 339 | 11 ms |
| `26/10` (sin año) | 4 | 10 ms |
| `2025` | 3983 | 6 ms |
| `005` | 1100 | 4 ms |
| `marzo 2025` | 338 | 1,2 ms |
| `img_05000` | 1 | 0,5 ms |
| `nada` | 0 | 0,3 ms |

Las palabras que casan con todas las fotos son el peor caso, porque se cuentan todas para la barra de meses.

### Favoritos y archivo en el núcleo, 10 000 fotos (2026-11-03)
Spec 006, tareas 2 a 4. Mismo equipo. El objetivo es menos de 1 s.

| Acción sobre las 10 000 fotos | Tiempo |
|---|---|
| Añadir a Favoritos | 35 ms |
| Archivar | 36 ms |

Con las 10 000 en Favoritos y archivadas (el peor caso para esas vistas), los meses y la primera página de 500 de Favoritos tardan 13 ms y los de Archivo 12 ms. La vista Fotos sigue en 7 ms para los meses y 1,5 ms la página más lenta. Para la selección (tarea 4), «seleccionar todo» tarda 0,8 ms y el rango de la primera foto a la última, 1,6 ms.

### Etiquetas en el núcleo, 10 000 fotos y 500 etiquetas (2026-11-12)
Spec 016, tarea 2. Solo filas en la base de datos (las etiquetas no abren archivos), en el contenedor de desarrollo y en modo release: `cargo test --release -p mnemos-core --test rendimiento_etiquetas -- --ignored --nocapture`. Falta medirlo en el HDD, donde cada transacción escribe en disco.

| Acción | Tiempo | Objetivo |
|---|---|---|
| Poner una etiqueta a las 10 000 | 75 ms | menos de 300 ms |
| Quitarla a las 10 000 | 103 ms | menos de 300 ms |
| Renombrar una etiqueta con 10 000 fotos | 119 ms | menos de 300 ms |
| Listar 500 etiquetas con recuento y portada | 36 ms | menos de 50 ms |
| Uso de las etiquetas en 10 000 seleccionadas | 25 ms | menos de 50 ms |
| Buscar el nombre de la etiqueta (10 000 resultados) | 11 ms | menos de 100 ms |

### Papelera en el núcleo, 10 000 fotos (2026-11-13)
Spec 015, tarea 3. Contenedor de desarrollo, en modo release. El objetivo es menos de 1 s.

| Acción sobre las 10 000 fotos | Tiempo |
|---|---|
| Enviar a la Papelera | 30 ms |
| Restaurar | 30 ms |

Con las 10 000 en la Papelera (su peor caso), los meses y la primera página de la vista Papelera tardan 4,7 ms.

### Interfaz, 10 000 fotos
| Medida | Resultado | Objetivo |
|---|---|---|
| Disponer toda la biblioteca (vitest) | unos 20 ms | — |
| Buscar lo visible en un fotograma (vitest) | menos de 0,05 ms | — |
| Desplazamiento continuo | 60 fps; el peor fotograma, 17 ms | 60 fps |
| Abrir el visor hasta pintar la foto | 71 ms | menos de 100 ms |
| Pasar a la siguiente foto | entre 29 y 34 ms | menos de 100 ms |
| Celdas en el DOM | unas 60, sin importar el tamaño de la biblioteca | — |

## Miniaturas de fotos grandes (2026-10-27)
Los JPEG y las vistas previas RAW se decodifican ya reducidos a 1/2, 1/4 o 1/8 en la IDCT (`jpeg-decoder`), al menor tamaño que cubre la miniatura. Fotos generadas con grano de sensor; mediana de 5 repeticiones por foto, un hilo, en el mismo contenedor.

| Foto | Antes | Después |
|---|---|---|
| 12 MP de móvil (4032 × 3024) | 195 ms | 66 ms |
| 24 MP, submuestreo 4:2:0 | 364 ms | 132 ms |
| 24 MP, 4:4:4 | 628 ms | 285 ms |
| 24 MP progresivo | 672 ms | 382 ms |
| 24 MP en gris | 157 ms | 89 ms |
| 50 MP | 670 ms | 200 ms |
| 6 MP en CMYK (sin reducción) | 186 ms | 186 ms |

La miniatura reducida y la completa difieren en unos 44 dB de PSNR: no se distinguen a simple vista. Para repetirlo con fotos reales: `cargo run --release -p mnemos-imaging --example medir_miniaturas -- <carpeta> [repeticiones] [carpeta_salida]`.

## Fotos editadas (spec 009, 2026-11-05)
Imagen del visor (lado mayor de 2560 px) y miniatura de una foto sintética de 24 MP (6000 × 4000) con la receta aplicada. Mediana de 5 repeticiones, un hilo, en el contenedor de 4 núcleos. Para repetirlo: `cargo run --release -p mnemos-imaging --example medir_ediciones -- [foto] [repeticiones]`.

| Edición | Visor, primera versión | Visor | Miniatura |
|---|---|---|---|
| Sin editar | 411 ms | 370 ms | 59 ms |
| Giro de 90° | 483 ms | 407 ms | 64 ms |
| Giro y recorte del 80 % | 877 ms | 616 ms | 54 ms |
| Recorte del 25 % | 147 ms | 134 ms | 61 ms |

- La orientación EXIF, el giro y el volteo se juntan en una sola transformación que se aplica después de recortar y reducir, sobre la imagen pequeña: girar no cuesta casi nada.
- Un recorte pequeño se decodifica a más resolución que la foto entera, porque tiene que cubrir el tamaño final. Con un recorte del 80 % el JPEG ya no se puede decodificar a la mitad y se lee entero.
- Casi todo el tiempo se va en el filtro de reducción de `image` (unos 200 ms para pasar de 3000 a 2560 px). Si hace falta bajar más, la opción es una librería de reducción con SIMD (como `fast_image_resize`), que sería una dependencia nueva a revisar.

## Pendiente
- Repetir el indexado y las miniaturas desde un HDD externo con fotos reales de cámara de 12 a 24 MP. Las sintéticas son pequeñas y el disco del contenedor es rápido, así que las miniaturas reales tardarán más por foto.
- Medir el visor dentro de la aplicación Tauri (WebView2, WKWebView, WebKitGTK) y no solo en Chromium.

## Duplicados exactos (spec 010, 2026-11-03)
`cargo test --release -p mnemos-core --test rendimiento duplicados -- --ignored --nocapture`. Filas insertadas directamente en la base de datos, una de cada diez con la huella repetida (1000 grupos). Mismo contenedor.

| Paso | Tiempo |
|---|---|
| Resumen y primera página de 50 grupos | 4,5 ms |
| Recorrer los 1000 grupos de 50 en 50 | 49 ms en total |
