# 007 · Metadatos de Google Takeout

- Estado: en curso
- Fase: F2

## Qué y por qué
Vengo de Google Fotos. Al importar mi Takeout quiero conservar lo que ya había
hecho allí: las descripciones que escribí, dónde se hizo cada foto, cuáles eran
favoritas, cuáles había archivado y en qué álbumes estaban. Hoy MNEMOS solo lee
la fecha de esos `.json` (spec 004) y todo lo demás se pierde.

Sale de la comparativa con Google Fotos (prioridad 1). Esta spec solo toca el
núcleo: la interfaz mostrará la descripción y el lugar en el panel de
información (spec 011) y los álbumes en su propia spec.

## Formato de Takeout
Cada foto lleva al lado un `.json` (los nombres y sus variantes ya los resuelve
la spec 004). Los campos que se leen:

```json
{
  "title": "IMG_20190512_143000.jpg",
  "description": "Subida al Naranjo",
  "photoTakenTime": { "timestamp": "1557664200" },
  "geoData":     { "latitude": 43.25, "longitude": -4.81, "altitude": 2519.0 },
  "geoDataExif": { "latitude": 43.25, "longitude": -4.81, "altitude": 2519.0 },
  "favorited": true,
  "archived": true
}
```

- `geoData` es la ubicación que muestra Google (puede estar corregida a mano);
  `geoDataExif`, la que traía la foto. Sin ubicación, Google escribe `0.0` en
  todo.
- `favorited` y `archived` solo aparecen cuando valen `true`.
- Una carpeta de álbum lleva además un `.json` con los datos del álbum, con un
  nombre que depende del idioma de la cuenta (`metadata.json`,
  `metadatos.json`…):

  ```json
  { "title": "Picos de Europa 2019", "description": "", "access": "protected",
    "date": { "timestamp": "1557664200" } }
  ```

  Las carpetas por año (`Photos from 2019`, `Fotos de 2019`) no son álbumes.
  En un Takeout la misma foto aparece en la carpeta del año y en la de cada
  álbum, con el mismo contenido.

**Contrastado** el 2026-11-05 con el Takeout real de Paulo: el `.json` del
álbum se llama `metadatos.json` y trae `title`, `description`, `access` y
`date`; las carpetas por año se llaman `Fotos del AAAA`. En una muestra de 49
fotos, 10 traían `archived: true`, 4 `geoDataExif` y 34 ubicación; ninguna
traía `favorited` ni descripción. Las claves `appSource` y
`removeResultReason` se ignoran.

## Comportamiento
- Al importar, cada foto con `.json` de Takeout guarda además de la fecha:
  - **Descripción**: `description` sin espacios sobrantes, si no está vacía.
  - **Ubicación**: latitud, longitud y altitud de `geoData` o, si está a cero,
    de `geoDataExif`. Una ubicación fuera de rango (latitud > 90, longitud >
    180) o exactamente `0, 0` se ignora.
  - **Favorita**: `favorited: true` la marca como favorita (columna de la spec
    006).
  - **Archivada**: `archived: true` la archiva con la fecha de la importación.
  - **Álbumes**: si la carpeta de la foto es un álbum, la foto entra en un
    álbum de MNEMOS con el mismo título. Si ya existe un álbum con ese nombre,
    se usa ese; si no, se crea.
- El `.json` de la foto se lee una sola vez y sirve para la fecha y para todo lo
  demás.
- **Una sola vez por foto:** descripción, ubicación, favorita y archivada se
  aplican la primera vez que se encuentra el `.json` de esa foto y se anota
  cuándo (`photos.takeout_at`). Volver a importar no deshace lo que el usuario
  cambie después en MNEMOS (quitar una favorita, desarchivar).
- **Repetidas:** si la foto ya está en la biblioteca (misma huella), no se
  copia, pero se aplican sus metadatos si aún no tenía (`takeout_at` vacío) y se
  añade al álbum de su carpeta. Así, volver a importar la carpeta del Takeout
  completa las 1401 fotos ya importadas, y la copia de una foto en un álbum la
  añade a ese álbum aunque ya se hubiera importado desde la carpeta del año.
- Nada se escribe en los archivos de imagen: todo va a la base de datos.
- `mnemos index` no lee `.json` (no están dentro de la biblioteca).

## Casos límite
- `.json` dañado, enorme (> 1 MiB) o sin los campos: se ignora lo que falte,
  la foto se importa igual.
- `description` con solo espacios o de más de 10 000 caracteres: se ignora o se
  corta.
- Ubicación con `latitude`/`longitude` que no son números: se ignora.
- Álbum con título vacío: la carpeta no cuenta como álbum.
- Varios álbumes de Google con el mismo nombre: acaban en un solo álbum de
  MNEMOS. Takeout no da un identificador estable del álbum.
- Una foto en dos álbumes: entra en los dos.
- Una foto archivada sigue en sus álbumes: el álbum tiene las mismas fotos que
  en Google Fotos, archivadas incluidas, y la vista de álbumes las mostrará
  (decidido por Paulo el 2026-11-05).
- Altitud `0.0`: Google la escribe cuando no la conoce; se guarda vacía.
- Fotos que no son de Takeout (sin `.json`): todo igual que antes.

## Criterios de aceptación
- [x] Una foto con `.json` importa su descripción, ubicación, favorita y archivo.
- [x] `geoData` a cero usa `geoDataExif`; ambos a cero dejan la ubicación vacía.
- [x] Una foto en una carpeta con `metadata.json`/`metadatos.json` entra en el
      álbum con ese título; la carpeta del año no crea álbum.
- [x] Reimportar completa los metadatos de las fotos ya importadas sin
      copiarlas y sin duplicar álbumes ni entradas.
- [x] Reimportar no vuelve a marcar como favorita una foto que el usuario quitó.
- [x] Una foto de un álbum que ya estaba importada desde la carpeta del año se
      añade al álbum.
- [ ] Probado con el Takeout real de Paulo (número de fotos con descripción,
      ubicación, favoritas, archivadas y álbumes, comparado con Google Fotos).
- [x] Los originales y el origen no se modifican (prueba automática).

## Fuera de alcance
- Mostrar descripción, ubicación o álbumes en la interfaz (specs 011 y álbumes).
- Buscar por la descripción (con las etiquetas en FTS).
- Personas (`people`), vistas (`imageViews`), origen del dispositivo.
- La papelera (`trashed`): las fotos en la papelera de Google se importan como
  las demás hasta que exista la spec de la papelera.
- Fotos compartidas, comentarios y títulos de recuerdos (`shared_album_comments.json`,
  `user-generated-memory-titles.json`).
- Leer el GPS del EXIF de fotos que no son de Takeout (spec del panel de
  información).

## Tareas (≤1 h cada una)
1. [x] Contrastar el formato con el Takeout real y ajustar esta spec.
2. [x] Migración 0008: `description`, `latitude`, `longitude`, `altitude`,
       `takeout_at` en `photos`.
3. [x] Leer el `.json` completo de cada foto y el del álbum de cada carpeta
       (módulo `takeout`).
4. [x] Importación: aplicar los metadatos a fotos nuevas y repetidas, álbumes
       sin duplicar, y pruebas.
