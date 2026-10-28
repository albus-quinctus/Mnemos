# Arquitectura

> Borrador. Las decisiones firmes se registran en `docs/decisiones/`.

## Capas
| Capa | Ubicación | Responsabilidad |
|---|---|---|
| Interfaz | `apps/desktop/src` (Svelte 5) | Cuadrícula, visor, álbumes, editor |
| Puente | `apps/desktop/src-tauri` | Comandos Tauri, permisos mínimos |
| Núcleo | `crates/mnemos-core` | Biblioteca, base de datos, indexado, búsqueda |
| Imagen | `crates/mnemos-imaging` | Miniaturas, EXIF, aplicación de recetas de edición |
| CLI | `crates/mnemos-cli` | Reindexar, verificar integridad, exportar |

## Formato de la biblioteca en el disco
```
DISCO/
├── MNEMOS-windows.exe, MNEMOS-macos.app, MNEMOS-linux.AppImage
└── Biblioteca/
    ├── Originales/AAAA/MM/...        (nunca se modifican)
    └── .mnemos/
        ├── mnemos.db                 (SQLite, rutas relativas)
        ├── miniaturas/
        ├── ediciones/                (recetas JSON versionadas)
        ├── copias/
        └── tmp/                      (copias a medias durante la importación)
```

## Reglas
- Solo rutas relativas a `Biblioteca/`.
- Edición no destructiva: la imagen editada se genera a partir del original y la receta.
- Sistema de archivos del disco: exFAT (spec 005; guía en `docs/guias/pasar-el-disco-a-exfat.md`).
