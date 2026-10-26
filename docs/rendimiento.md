# Rendimiento

Mediciones de la spec 002 (tarea 15) con bibliotecas sintéticas. Nunca se usa una biblioteca real.

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

### Interfaz, 10 000 fotos
| Medida | Resultado | Objetivo |
|---|---|---|
| Disponer toda la biblioteca (vitest) | unos 20 ms | — |
| Buscar lo visible en un fotograma (vitest) | menos de 0,05 ms | — |
| Desplazamiento continuo | 60 fps; el peor fotograma, 17 ms | 60 fps |
| Abrir el visor hasta pintar la foto | 71 ms | menos de 100 ms |
| Pasar a la siguiente foto | entre 29 y 34 ms | menos de 100 ms |
| Celdas en el DOM | unas 60, sin importar el tamaño de la biblioteca | — |

## Pendiente
- Repetir el indexado y las miniaturas desde un HDD externo con fotos reales de cámara de 12 a 24 MP. Las sintéticas son pequeñas y el disco del contenedor es rápido, así que las miniaturas reales tardarán más por foto.
- Medir el visor dentro de la aplicación Tauri (WebView2, WKWebView, WebKitGTK) y no solo en Chromium.
