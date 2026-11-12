# 012 · Acceso directo en el equipo

- Estado: en curso
- Fase: F1 (completa la versión portátil de la spec 005)

## Qué y por qué
Quiero abrir MNEMOS desde el menú de aplicaciones de mi ordenador, como
cualquier otro programa, sin buscar el ejecutable en el disco cada vez. En
Linux es lo que más falta: casi ningún gestor de archivos ejecuta un AppImage
con doble clic.

## Qué es posible y qué no
- **Un mismo archivo para los tres sistemas no existe.** Cada sistema tiene su
  formato de acceso directo, y todos guardan una ruta absoluta.
- **Un acceso directo dentro del disco no sirve.** Un `.lnk` de Windows guarda la
  letra (`E:`), que cambia de un equipo a otro. En Linux, un `.sh` o un
  `.desktop` en un disco externo tiene el mismo problema que el AppImage: el
  gestor de archivos lo abre en un editor o pide marcarlo como de confianza en
  cada equipo. Windows no ejecuta nada al enchufar un USB (AutoRun desactivado).
- **Sí funciona crear el acceso directo en el equipo**, desde la propia app y
  solo cuando el usuario lo pide. Vale para ese equipo; en otro se vuelve a
  pulsar el botón.
- **En Windows y macOS el doble clic ya funciona** (`MNEMOS.exe` y `MNEMOS.app`
  en la raíz del disco). En macOS basta con arrastrar la app al Dock, que la
  sigue encontrando aunque cambie el punto de montaje, así que la app no crea
  nada allí.

## Comportamiento
- Solo en **modo portátil** (spec 005) y solo en **Linux y Windows**. Instalada,
  ya tiene su acceso directo; en macOS, el botón no aparece y en su lugar el
  `LEEME.txt` explica lo del Dock.
- Al pie del menú lateral, bajo las utilidades, una entrada
  **«Añadir a este equipo»**. Al pulsarla:
  - **Linux:** crea `~/.local/share/applications/mnemos-portatil.desktop`
    (o en `$XDG_DATA_HOME/applications/` si está definida) y copia el icono a
    `~/.local/share/icons/hicolor/256x256/apps/mnemos-portatil.png`. El
    `.desktop` lleva `Exec` y `TryExec` con la ruta del AppImage
    (variable `APPIMAGE`): con el disco desenchufado, los lanzadores que
    respetan `TryExec` ocultan la entrada. Aparece en fuzzel, wofi, rofi,
    GNOME, KDE y XFCE sin reiniciar la sesión en la mayoría de ellos.
  - **Windows:** crea `MNEMOS.lnk` en el menú Inicio
    (`%APPDATA%\Microsoft\Windows\Start Menu\Programs\`) y en el escritorio,
    con el icono del propio `MNEMOS.exe` y la carpeta del disco como carpeta
    de trabajo.
  - Aviso breve: «MNEMOS está en el menú de este equipo».
- Si ya existe, la entrada pasa a **«En este equipo · Quitar»**. Quitar borra
  solo los archivos que creó MNEMOS (los reconoce por una marca propia:
  `X-Mnemos-Portatil=1` en el `.desktop`; la descripción del `.lnk`) y nunca
  otro archivo con el mismo nombre.
- **Se corrige solo:** cada vez que MNEMOS arranca en modo portátil y encuentra
  su acceso directo apuntando a otra ruta (otra letra de unidad, otro punto de
  montaje), lo reescribe con la ruta actual. No crea ninguno si no existe.
- Es la única excepción a «no dejar rastro» de la spec 005, y solo ocurre si el
  usuario pulsa el botón.
- `LEEME.txt` se actualiza: en macOS 15 o posterior, «clic derecho → Abrir» ya
  no basta para una app sin firmar; hay que intentar abrirla, ir a Ajustes del
  Sistema → Privacidad y seguridad y pulsar «Abrir igualmente». Se añade lo del
  Dock en macOS y el botón en Linux y Windows.

## Casos límite
- Rutas con espacios, tildes, comillas o `%` en el punto de montaje: el `Exec`
  del `.desktop` se escapa según la especificación de freedesktop (entre
  comillas dobles, con `\` delante de `"`, `` ` ``, `$` y `\`, y `%` como `%%`).
- `HOME` o `APPDATA` no definidos, o carpeta sin permiso de escritura: mensaje
  claro, no se crea nada a medias.
- Ya existe un `mnemos-portatil.desktop` o `MNEMOS.lnk` que no es nuestro: no se
  sobrescribe y se avisa.
- Dos discos con MNEMOS en el mismo equipo: el acceso directo apunta al último
  que lo creó o lo corrigió. Se acepta.
- Linux sin `APPIMAGE` (ejecutable suelto o `--appimage-extract-and-run`): se
  usa la ruta del ejecutable.
- El disco se monta en solo lectura: el acceso directo se crea igual, porque se
  escribe en el equipo, no en el disco.

## Criterios de aceptación
- [ ] En Linux, el `.desktop` generado es válido (`desktop-file-validate` en el
      CI) y abre MNEMOS desde fuzzel en Sway con el HDD real.
- [x] El escapado de `Exec` funciona con espacios, tildes, comillas y `%`
      (prueba automática).
- [x] Al cambiar la ruta del disco, el acceso directo se reescribe al arrancar;
      si no existía, no se crea (prueba automática con carpetas temporales).
- [x] «Quitar» borra solo archivos con la marca de MNEMOS (prueba automática).
- [ ] En Windows se crean los dos `.lnk` y abren la app (guía de prueba manual).
- [ ] Sin modo portátil o en macOS, la entrada no aparece.
- [ ] Los originales no se modifican (prueba automática).

## Fuera de alcance
- Crear algo en macOS (Dock, Launchpad o alias).
- Asociar tipos de archivo (abrir fotos con MNEMOS desde el gestor de archivos).
- Iconos en el escritorio de Linux: no hay un escritorio común a todos los
  entornos.
- Cambiar el gestor de archivos para que ejecute el AppImage con doble clic.

## Tareas (≤1 h cada una)
1. [x] Módulo `acceso_directo` (Rust): rutas de destino por sistema, contenido
       del `.desktop` con escapado y marca, lectura de la ruta actual; pruebas.
2. [x] Crear, quitar y corregir en Linux, con pruebas en carpetas temporales
       (`XDG_DATA_HOME` falso).
3. [ ] `.lnk` en Windows: elegir entre una dependencia pequeña revisada con
       `cargo deny` o PowerShell (`WScript.Shell`); crear, quitar y corregir.
4. [ ] Comandos de Tauri `acceso_directo_estado`, `crear_acceso_directo`,
       `quitar_acceso_directo` y corrección al arrancar; envoltorios en
       `biblioteca.ts`.
5. [ ] Capturas del diseño de la entrada del menú lateral (claro y oscuro) para
       que Paulo las apruebe.
6. [ ] Entrada en `MenuLateral.svelte` y aviso; pruebas.
7. [ ] `LEEME.txt` (macOS 15, Dock, botón) y `desktop-file-validate` en el CI.
8. [ ] Prueba con el HDD real en Linux y guía de prueba en Windows.
