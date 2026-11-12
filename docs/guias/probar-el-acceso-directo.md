# Probar el acceso directo en el equipo

Prueba a mano de la spec 012 (`docs/specs/012-acceso-directo.md`): el botón
«Añadir a este equipo» del menú lateral. Las pruebas automáticas ya cubren la
lógica (rutas, escapado, marca de MNEMOS, corrección) y que el `.desktop` pasa
`desktop-file-validate`; esto comprueba lo que no se puede automatizar: el
disco real, el gestor de archivos y el menú de aplicaciones de cada sistema.

## Antes de empezar

1. Consigue el zip portátil más reciente. En GitHub: **Actions → Versión
   portátil → Run workflow** sobre `develop`, y al terminar descarga el
   artefacto `MNEMOS-portatil` (un zip dentro de otro zip: descomprime el
   primero).
2. Descomprime el zip en la **raíz del disco** (exFAT), junto a la carpeta
   `MNEMOS/` de la biblioteca. Si ya había una versión anterior, sustituye los
   ejecutables y deja `mnemos-portatil.json`, `.mnemos-portatil/` y `MNEMOS/`.
3. Expulsa el disco antes de desenchufarlo, siempre.

El acceso directo se crea en el ordenador, no en el disco. Para repetir una
prueba desde cero, usa «Quitar» o borra a mano los archivos que se indican.

## Linux (con el disco real)

Se prueba con el disco `FOTOS` en `/media/lilith/FOTOS` y Sway; los pasos
valen igual en GNOME o KDE cambiando el lanzador.

1. Abre `MNEMOS-linux.AppImage` (clic derecho → «Ejecutar», o desde una
   terminal). Se abre la biblioteca.
2. **Entrada visible.** Abajo en el menú lateral debe verse «Añadir a este
   equipo» con la pista «MNEMOS aparecerá en el menú de aplicaciones.».
3. Pulsa «Añadir a este equipo». Debe salir el aviso «MNEMOS está en el menú de
   este equipo» con «Quitar», y la entrada pasa a «✓ En este equipo · Quitar».
4. Comprueba los archivos:
   ```
   cat ~/.local/share/applications/mnemos-portatil.desktop
   ls ~/.local/share/icons/hicolor/256x256/apps/mnemos-portatil.png
   desktop-file-validate ~/.local/share/applications/mnemos-portatil.desktop
   ```
   `Exec` y `TryExec` deben llevar la ruta del `.AppImage` del disco, no una
   carpeta `/tmp/.mount_…`, y debe verse la línea `X-Mnemos-Portatil=1`.
5. **Lanzador.** Cierra MNEMOS y ábrelo desde el lanzador de aplicaciones (en
   Sway, fuzzel o wofi: escribe «MNEMOS»). Debe verse con su icono y abrir la
   biblioteca.
6. **Disco desenchufado.** Expulsa el disco y busca «MNEMOS» en el lanzador:
   no debería aparecer (por `TryExec`). Si aparece en el tuyo y falla al
   abrirlo, apunta qué lanzador es.
7. **Otro punto de montaje.** Vuelve a conectar el disco, y si es posible
   móntalo en otra ruta (por ejemplo, desmontarlo y montarlo a mano en
   `/mnt/fotos`). Abre MNEMOS desde ahí, cierra, y mira otra vez el `.desktop`:
   `Exec` y `TryExec` deben apuntar a la ruta nueva (se corrige solo al
   arrancar).
8. **Quitar.** Pulsa «Quitar». Debe salir «Se ha quitado MNEMOS de este
   equipo», desaparecer el `.desktop` y el icono, y volver «Añadir a este
   equipo». Repite el paso 3 y quita desde el botón «Quitar» del aviso.
9. **Archivo ajeno.** Crea un archivo falso y abre MNEMOS:
   ```
   printf '[Desktop Entry]\nName=Otro\nExec=otro\n' > ~/.local/share/applications/mnemos-portatil.desktop
   ```
   La entrada debe verse desactivada, con el aviso en rojo, y el archivo no
   debe cambiar. Bórralo al terminar.
10. **Instalada.** Abre MNEMOS sin el modo portátil (por ejemplo, desde una
    copia del ejecutable sin `mnemos-portatil.json` al lado): la entrada no
    debe aparecer.

## Windows

Hace falta un Windows 10 u 11 con el disco exFAT. Hasta ahora, la parte que
crea el `.lnk` con PowerShell solo se ha comprobado que compila: esta prueba es
la primera vez que se ejecuta.

1. Abre `MNEMOS.exe` desde el disco (si Windows protege el PC, «Más
   información» → «Ejecutar de todas formas»).
2. Abajo en el menú lateral debe verse «Añadir a este equipo» con la pista
   «MNEMOS aparecerá en Inicio y en el escritorio.». No debe parpadear ninguna
   ventana negra al abrir la app.
3. Púlsalo. Debe salir el aviso con «Quitar» y verse «✓ En este equipo».
4. Comprueba que hay un `MNEMOS.lnk` en el escritorio y otro en el menú Inicio
   (escribe «MNEMOS» en Inicio). Clic derecho → Propiedades: «Destino» es el
   `.exe` del disco, «Iniciar en» es la carpeta del disco y el icono es el de
   MNEMOS. Si el escritorio o Inicio están en OneDrive, deben estar ahí.
5. Cierra MNEMOS y ábrelo con cada acceso directo.
6. **Otra letra.** Cambia la letra de la unidad (Administración de discos →
   clic derecho en el disco → «Cambiar la letra y las rutas de acceso de
   unidad»). Abre MNEMOS con el `.exe` desde la letra nueva, ciérralo y mira el
   «Destino» de los dos accesos: debe tener la letra nueva.
7. **Rutas con tildes.** Si tu usuario o tu carpeta de escritorio tienen tilde
   o espacios, comprueba que el paso 4 sigue bien (es lo que más puede romper
   la lectura de PowerShell).
8. **Quitar.** Pulsa «Quitar»: deben desaparecer los dos `.lnk`.
9. **Archivo ajeno.** Crea un acceso directo llamado `MNEMOS.lnk` en el
   escritorio con otro destino (clic derecho → Nuevo → Acceso directo), abre
   MNEMOS y comprueba que la entrada sale desactivada y que el acceso no se
   toca.

## Qué contar al terminar

Para cada paso: «bien», o qué pasó y en qué sistema. Lo más útil si falla algo:
el texto exacto del aviso en rojo y, en Linux, la salida de
`journalctl --user -n 50` o de lanzar el AppImage desde una terminal (los
errores del arranque salen por ahí con el prefijo `acceso directo:`).

## macOS

No hay nada que probar: la app no crea nada ahí y el `LEEME.txt` explica cómo
dejarla en el Dock.
