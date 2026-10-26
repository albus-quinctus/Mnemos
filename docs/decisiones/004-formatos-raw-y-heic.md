# 004 · Librerías para RAW, HEIC y AVIF

- Fecha: 2026-10-26
- Estado: propuesta

## Contexto
La spec 002 pide miniaturas y visor para fotos de cualquier dispositivo. Para JPEG, PNG, WebP, GIF, BMP y TIFF basta con el crate `image` en Rust puro. Faltan dos grupos:
- **RAW** (DNG, CR2, CR3, NEF, NRW, ARW, RAF, ORF, RW2, PEF y SRW). Solo se usa la vista previa JPEG que trae el archivo; no se revela.
- **HEIC, HEIF y AVIF**, que hoy solo se pueden decodificar con librerías en C.

Condiciones: licencia compatible con GPL-3.0-or-later, funcionar en Windows, macOS y Linux con un ejecutable portátil, y no añadir código en C si se puede evitar.

## Decisión

### RAW: extractor propio de vistas previas en `mnemos-imaging`
- Un módulo pequeño en Rust seguro, sin `unsafe` y sin dependencias nuevas, que localiza la vista previa JPEG más grande del archivo y **devuelve sus bytes tal cual**, sin decodificarlos.
- Cubre tres familias de contenedor:
  - **Basados en TIFF** (DNG, CR2, NEF, NRW, ARW, ORF, PEF, SRW y RW2): recorre los IFD y las SubIFD buscando `JPEGInterchangeFormat` o tiras JPEG. En RW2 se usa la etiqueta `JpgFromRaw` (0x2E).
  - **RAF**: la cabecera de Fujifilm indica el desplazamiento y la longitud del JPEG.
  - **CR3** (ISO-BMFF): la caja `PRVW` del UUID de previsualización de Canon.
- La orientación se lee de la misma estructura TIFF y EXIF.
- Como los bytes son un JPEG normal, el visor los puede mostrar directamente, y la miniatura se genera con el mismo código que para cualquier JPEG.
- Cada familia se prueba con archivos pequeños en `tests/fixtures/`, de licencia libre y sin datos personales, además de pruebas de archivos truncados o corruptos, que nunca deben provocar un pánico.

### HEIC, HEIF y AVIF: `libheif` compilado dentro del ejecutable, tras una prueba
- Se usa `libheif-rs` (MIT) con la opción `embedded-libheif`, que compila `libheif` (LGPL-3.0) y sus decodificadores `libde265` para HEVC y `dav1d` para AV1, y los enlaza dentro del ejecutable. No hace falta instalar nada en el equipo del usuario.
- Antes de la tarea 14 de la spec 002 se hace una prueba aislada: compilar y decodificar un HEIC y un AVIF en el CI de los tres sistemas. Si la prueba falla o añade más de 5 minutos al CI, se vuelve a esta decisión.
- Hasta entonces, estas fotos se muestran con el marcador "Sin vista previa".

#### Resultado de la prueba (2026-10-26)
- Rama `prueba/libheif`, con el flujo `.github/workflows/prueba-libheif.yml` y el crate `prueba-libheif/`. La rama no se fusiona.
- `embedded-libheif` **no basta**: compila `libheif`, pero enlaza `libde265` y `dav1d` como librerías del sistema.
- Lo que funciona en los tres sistemas: `libheif` 1.23.5 de vcpkg (etiqueta 2026.10.16) como librería estática, con `libde265` para HEIC y `aom` (BSD-2-Clause) para AVIF, porque `dav1d` aún no está en una versión publicada de vcpkg. Los triplets son `x64-linux`, `arm64-osx` y `x64-windows-static-md`. El ejecutable decodifica un HEIC y un AVIF sin depender de nada instalado en el equipo.
- Tiempo en el CI: sin caché, vcpkg tarda unos 6 min en Linux, 5 en macOS y 10 en Windows. Con la caché de vcpkg, cada trabajo tarda alrededor de 1 min en total.
- **Aplazada** por Paulo el 2026-10-26: HEIC y AVIF siguen sin miniatura por ahora. La decisión sigue como propuesta, para retomarla con estos datos.

## Alternativas consideradas
- **`rawler`** (dnglab, LGPL-2.1, Rust puro). Es la opción más completa: conoce casi todos los RAW y en el futuro podría servir para revelarlos. En contra:
  - Trae dependencias pesadas como `rayon`, `jxl-oxide` y `image` con sus propias opciones.
  - Su `preview_image` decodifica el JPEG, que habría que volver a codificar para el visor.
  - No da vista previa de ORF ni de SRW.
  Queda como candidata para el revelado de F3.
- **`rawloader`, `quickraw`** (LGPL-2.1): están pensadas para revelar, no para extraer la vista previa; `quickraw` lleva sin actualizarse desde 2023.
- **Crate `heic`** (Rust puro, AGPL-3.0 o licencia comercial). Es muy reciente (0.1) y su AGPL complica la licencia del conjunto. Se reconsiderará si madura.
- **Decodificadores del sistema** (ImageIO en macOS, WIC en Windows). En Windows HEIC necesita una extensión de pago y en Linux no existen, así que el resultado dependería del equipo. Es el plan B si `libheif` no compila bien en el CI.
- **Decodificar AVIF con `image`** (`avif-native`, que usa `dav1d`). También es C y no cubre HEIC; `libheif` resuelve los dos a la vez.

## Consecuencias
- MNEMOS mantiene su propio código para las vistas previas RAW: unas pocas centenas de líneas que hay que probar bien con archivos reales de cada marca.
- Con `libheif` entra código en C y, con `libde265`, un decodificador HEVC. HEVC tiene patentes cuyo cumplimiento en la distribución binaria depende del país. Proyectos libres como GIMP o darktable lo distribuyen igual, pero conviene mencionarlo en las notas de la versión. Hay que avisar también de que el ejecutable contiene código LGPL-3.0, compatible con la GPL-3.0-or-later.
- `deny.toml` no ve las licencias de las librerías en C, así que se anotan a mano en `THIRD-PARTY.md` cuando se añadan.
