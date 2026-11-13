# 013 · Ajustes y pantalla completa

- Estado: en curso
- Fase: F2 (personalización de la interfaz)

## Qué y por qué
Quiero elegir si MNEMOS se ve claro u oscuro sin depender del tema del
sistema, y tener un sitio fijo donde irán las demás preferencias. También
quiero ver las fotos a pantalla completa con `F11`, como en un navegador o un
visor de imágenes. Lo pidió Paulo el 2026-11-12.

## Comportamiento

### Ajustes
- En el menú lateral, al pie, una entrada **«Ajustes»** con un icono de
  engranaje, justo **encima de «Añadir a este equipo»** (spec 012). Si esa
  entrada no aparece (instalada o en macOS), «Ajustes» queda igualmente al pie.
- Al pulsarla se abre una **ventana de Ajustes** centrada sobre la galería
  (diálogo modal, con fondo atenuado), sin cambiar de sección ni perder la
  posición de la cuadrícula. Se cierra con la cruz, con `Esc` o pulsando fuera.
  Los cambios se aplican al momento: no hay botón «Guardar».
- Contenido de la primera versión:
  - **Apariencia**: tres opciones en un control segmentado,
    **Sistema · Claro · Oscuro**. «Sistema» (por defecto) sigue la preferencia
    del sistema operativo, como hasta ahora, y cambia sola si el sistema cambia.
    El visor y el editor siguen oscuros en los tres casos (decisión 003).
  - **Pantalla completa**: un interruptor con la pista «También con F11».
    Refleja el estado real de la ventana, aunque se haya cambiado con la tecla.
  - Al pie, en pequeño, la versión de MNEMOS (`MNEMOS 0.0.1`).
- La ventana está pensada para crecer: cada grupo es una sección con su título,
  y las preferencias futuras (idioma, tamaño de las miniaturas, carpeta de
  exportación…) se añaden como secciones nuevas sin rediseñarla.

### Dónde se guarda el tema
- En el `configuracion.json` que ya guarda la última biblioteca (spec 002), con
  un campo nuevo `tema` (`"sistema"`, `"claro"` u `"oscuro"`). **No toca la base
  de datos ni necesita migración**: es una preferencia de la aplicación, no de
  la biblioteca.
- En modo portátil ese archivo vive en el disco (`.mnemos-portatil/`), así que
  el tema viaja con el disco a cualquier equipo; instalada, vive en la carpeta
  de configuración del sistema.
- Un `configuracion.json` antiguo sin `tema`, o con un valor desconocido, se lee
  como `"sistema"`. Guardar el tema no borra la última biblioteca, ni recordar
  la biblioteca borra el tema (hoy `recordar` reescribe el archivo entero:
  pasa a leerlo, cambiar su campo y guardarlo).
- **Sin destello al arrancar:** al crear la ventana, Rust lee el tema y lo
  inyecta antes de cargar la página (`initialization_script` que pone
  `data-tema` en `<html>`; si `<html>` aún no existe, espera a que aparezca
  con un `MutationObserver`), y ajusta el tema nativo de la ventana
  (`set_theme`) para que la barra de título y los menús del sistema
  coincidan. Con «Sistema» no se fuerza nada.
- En CSS, los tokens oscuros se aplican con
  `@media (prefers-color-scheme: dark)` solo si no hay `data-tema="claro"`, y
  siempre con `data-tema="oscuro"`; `color-scheme` sigue al tema elegido para
  que las barras de desplazamiento y los campos nativos también cambien.

### Pantalla completa
- `F11` alterna la pantalla completa de la ventana en cualquier pantalla:
  galería, búsqueda, visor y editor. No se pierde nada de lo que estaba abierto.
- Se hace desde Rust (`set_fullscreen`) con un comando
  `alternar_pantalla_completa` que devuelve el estado pedido (en Wayland el
  compositor la concede un instante después; `pantalla_completa` da el real); no hace falta
  ampliar los permisos de la ventana.
- `Esc` **no** sale de pantalla completa: ya cierra el visor, la búsqueda y la
  selección. Para salir, `F11` otra vez o el interruptor de Ajustes.
- La pantalla completa no se recuerda: cada arranque empieza en ventana.
- En macOS, `F11` suele estar reservada por el sistema (mostrar el escritorio);
  allí también vale `Ctrl` + `Cmd` + `F`, el atajo habitual del sistema.

## Casos límite
- `configuracion.json` dañado: se arranca con el tema del sistema y sin última
  biblioteca, como ya pasa hoy; al cambiar el tema se reescribe bien.
- Carpeta de configuración sin permiso de escritura (disco en solo lectura): el
  tema se aplica en la sesión y se avisa con «No se pudo guardar el ajuste»;
  no se bloquea nada.
- Cambiar el tema con el visor abierto: el visor no cambia (siempre oscuro) y
  la galería de detrás sí.
- Sway y otros gestores de ventanas en mosaico: la pantalla completa la concede
  el compositor (`xdg_toplevel.set_fullscreen`); si no la concede, el
  interruptor vuelve a su estado real.
- Pulsar `F11` mientras se escribe en la búsqueda o en el título del día:
  alterna igual y no escribe nada en el campo.

## Criterios de aceptación
- [ ] La entrada «Ajustes» aparece en el menú lateral encima de «Añadir a este
      equipo» y abre la ventana de Ajustes; `Esc` la cierra.
- [ ] Elegir Claro u Oscuro cambia toda la galería al momento, también si el
      sistema está en el modo contrario (prueba de componentes).
- [x] El tema se recuerda entre arranques y un `configuracion.json` sin `tema`
      se lee como «Sistema» (pruebas en Rust).
- [x] Guardar el tema conserva la última biblioteca y al revés (prueba en Rust).
- [ ] `F11` entra y sale de pantalla completa en la galería, el visor y el
      editor; el interruptor de Ajustes muestra el estado real (prueba de la
      interfaz con Tauri simulado; prueba manual en Sway).
- [ ] No hay destello claro al arrancar en oscuro (prueba manual).
- [ ] Ningún ajuste toca la base de datos ni los originales.

## Fuera de alcance
- Otras preferencias (idioma, tamaño de miniaturas, carpeta de exportación,
  atajos configurables): irán en specs propias como secciones de Ajustes.
- Recordar el tamaño y la posición de la ventana.
- Una vista de Ajustes como sección de página completa.
- Ocultar el menú lateral y las barras en pantalla completa (modo
  presentación o pase de diapositivas).

## Tareas (≤1 h cada una)
1. [x] `tema` en `Configuracion` (`serde(default)`, valor desconocido →
       `sistema`), `recordar` sin borrar otros campos, comando `poner_tema`
       (la página lee el tema de `data-tema`, así que no hace falta otro);
       inyección en la ventana y `set_theme`; pruebas en Rust
       (`configuracion.rs`, `Configuracion::cambiar`).
2. [x] Comandos `alternar_pantalla_completa` y `pantalla_completa`; atajo `F11`
       (y `Ctrl` + `Cmd` + `F` en macOS) en `+layout.svelte`; envoltorios en
       `biblioteca.ts`; pruebas (`lib/ajustes/pantalla.ts`).
3. [x] Capturas del diseño de la entrada y de la ventana de Ajustes (claro y
       oscuro), aprobadas por Paulo el 12 nov (`capturas/013-*.png`).
4. [x] Tokens con `data-tema` en `tokens.css` y estado del tema en
       `lib/ajustes/tema.svelte.ts`; pruebas (incluida una que comprueba
       que los dos bloques oscuros de `tokens.css` son iguales).
5. [ ] Entrada «Ajustes» en `MenuLateral.svelte` y ventana de Ajustes
       (`lib/ajustes/Ajustes.svelte`); pruebas.
6. [ ] Prueba en la app real (Linux con Sway; Windows si se puede) y
       `docs/progreso.md`.
