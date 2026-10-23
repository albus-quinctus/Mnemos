# Hoja de ruta

| Fase | Contenido | Criterio de salida | Skills clave |
|---|---|---|---|
| **F0 Fundaciones** | Monorepo, CI, docs, licencia, esquema SQLite inicial | CI en verde; ventana Tauri en los 3 sistemas | brainstorming, Tauri, open-sourcing |
| **F1 Biblioteca y navegación** (0.1 alfa) | Importar carpeta, indexar, miniaturas, cuadrícula, visor, búsqueda básica | 10 000 fotos fluidas desde un HDD externo | Svelte, rust-review, property-based-testing |
| **F2 Organización** (0.5 beta) | Álbumes, etiquetas, búsqueda FTS5, EXIF | Todo persistente y portátil entre equipos | insecure-defaults, skills propias EXIF/SQLite |
| **F3 Edición** (1.0) | Recorte, dibujo, anotaciones, recetas JSON y versiones | Pruebas que demuestran originales intactos | differential-review, property-based-testing |
| **F4 Avanzado** | Duplicados, copias de seguridad, formatos extra, personalización | Según prioridad | revisión de seguridad completa |

**Hitos de seguridad:** revisión completa antes de 0.5 y de 1.0; auditoría de dependencias en cada versión.
