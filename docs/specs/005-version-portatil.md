# 005 · Versión portátil en el disco

- Estado: borrador
- Fase: F1 (antes de la beta)

## Qué y por qué
Quiero enchufar mi disco en cualquier ordenador (Windows, macOS o Linux), hacer
doble clic en MNEMOS y ver mis fotos, sin instalar nada y sin dejar rastro en
ese equipo.

## Comportamiento
- El disco lleva en la raíz los ejecutables, la biblioteca y unas instrucciones:
  ```
  DISCO/
  ├── MNEMOS.exe                  Windows 10/11 (x86_64)
  ├── MNEMOS.app/                 macOS 11 o posterior (Intel y Apple Silicon)
  ├── MNEMOS-linux.AppImage       Linux (x86_64)
  ├── LEEME.txt                   cómo abrirlo en cada sistema
  ├── mnemos-portatil.json        marca el modo portátil
  ├── .mnemos-portatil/           ajustes y datos del navegador interno
  └── MNEMOS/                     la biblioteca (Originales/ y .mnemos/)
  ```
- **Modo portátil:** si junto al ejecutable hay un `mnemos-portatil.json`, la
  app guarda sus ajustes y los datos de su navegador interno (WebView) en
  `.mnemos-portatil/`, en el propio disco, y no en el perfil del usuario del
  equipo. Sin ese archivo funciona como una app instalada, igual que hoy.
  - Junto al ejecutable quiere decir: la carpeta del `.exe`; la del `.AppImage`
    (variable `APPIMAGE`, no la carpeta temporal donde se monta); la que
    contiene `MNEMOS.app`.
- **Abrir sola la biblioteca:** en modo portátil, la última biblioteca se
  guarda como ruta relativa a la raíz del disco, porque la letra (`E:`, `F:`) o
  el punto de montaje (`/media/<usuario>/…`, `/Volumes/…`) cambia de un equipo
  a otro. Si no hay ninguna guardada, abre `MNEMOS/` si es una biblioteca.
- **Disco de solo lectura:** si la biblioteca no se puede escribir (macOS con
  NTFS, disco protegido o marcado como dañado), la app lo dice con claridad en
  lugar de fallar al abrirla. Ver la decisión pendiente sobre el formato.
- **Publicación:** al etiquetar una versión (`v0.1.0`), el CI compila las tres
  y publica `MNEMOS-portatil-<versión>.zip` con la estructura de arriba, sin la
  biblioteca. Se descomprime en la raíz del disco.
- Sin actualizaciones automáticas ni conexiones de red (regla 4).

## Decisión pendiente: formato del disco
El HDD de prueba está en **NTFS**. Windows y Linux leen y escriben NTFS, pero
macOS solo lo lee. Para usar MNEMOS en un Mac habría que pasar el disco a
**exFAT** (lo recomendado en `docs/arquitectura.md`), lo que obliga a copiar
todo fuera, formatear y volver a copiar. Hasta decidirlo, en macOS la
biblioteca se abriría en solo lectura o no se abriría.

## Casos límite
- El disco se desconecta con la app abierta: la base de datos ya está en modo
  de escritura segura (decisión 002); al volver a abrir no se pierde nada.
- Dos equipos abren el mismo disco a la vez: no es posible (es un disco), pero
  si la base de datos está bloqueada, mensaje claro.
- Rutas con espacios o tildes en el punto de montaje.
- macOS bloquea apps sin firmar descargadas de internet (Gatekeeper): `LEEME.txt`
  explica abrirla con clic derecho → Abrir la primera vez.
- Linux sin FUSE (necesario para AppImage): `LEEME.txt` explica
  `--appimage-extract-and-run`.
- Windows sin WebView2 (Windows 10 antiguo): `LEEME.txt` enlaza la descarga de
  Microsoft; la app no la instala sola.

## Criterios de aceptación
- [ ] Con `mnemos-portatil.json`, los ajustes y los datos del WebView se crean en el disco y no en el perfil del usuario (prueba automática de la resolución de rutas).
- [ ] La última biblioteca se guarda como ruta relativa y se abre aunque cambie el punto de montaje (prueba automática).
- [ ] Sin biblioteca guardada, abre `MNEMOS/` junto al ejecutable.
- [ ] Sin `mnemos-portatil.json`, la app se comporta como hoy.
- [ ] Una biblioteca en solo lectura muestra un aviso en vez de un error.
- [ ] El CI genera el zip portátil para las tres plataformas al etiquetar.
- [ ] Probado en Linux con el HDD real; Windows y macOS quedan con guía de prueba manual.
- [ ] Los originales no se modifican (prueba automática).

## Fuera de alcance
- Firmar los ejecutables (Windows SmartScreen, notarización de Apple): cuesta dinero y cuentas de desarrollador.
- Formatear o convertir el disco.
- Ejecutables ARM para Windows y Linux.
- Incluir `mnemos-cli` en el disco (se puede añadir después en `herramientas/`).

## Tareas (≤1 h cada una)
1. [ ] Módulo `portatil` en la app Tauri: detectar el modo y resolver las carpetas de ajustes y datos (con pruebas).
2. [ ] Guardar y leer la última biblioteca como ruta relativa; abrir `MNEMOS/` si no hay ninguna.
3. [ ] Carpeta de datos del WebView en el disco (`data_directory` de la ventana).
4. [ ] Aviso de biblioteca de solo lectura.
5. [ ] Workflow de publicación: compilar en los tres sistemas y montar el zip.
6. [ ] `LEEME.txt` con instrucciones por sistema.
7. [ ] Prueba en el HDD real en Linux y guía de prueba para Windows y macOS.
