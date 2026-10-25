# MNEMOS

Biblioteca fotográfica personal, portátil y multiplataforma. Organiza, busca y edita tus fotos sin nube, directamente desde tu disco externo.

> Estado: fase F0 (fundaciones). Todavía no hay versión utilizable.

- Visión: [docs/vision.md](docs/vision.md)
- Hoja de ruta: [docs/roadmap.md](docs/roadmap.md)
- Cómo se desarrolla: [docs/workflow.md](docs/workflow.md)

## Desarrollo

Requisitos: Rust estable, Node 22, pnpm 10 y las [dependencias de sistema de Tauri](https://v2.tauri.app/start/prerequisites/).

```sh
cd apps/desktop
pnpm install
pnpm tauri dev                                      # aplicación de escritorio
cargo run -p mnemos-cli -- init BIBLIOTECA           # crea una biblioteca vacía
cargo run -p mnemos-cli -- import BIBLIOTECA ORIGEN  # copia fotos sin tocar el origen
cargo run -p mnemos-cli -- index BIBLIOTECA          # registra lo que hay en Originales/
cargo test --workspace                              # pruebas del núcleo
```

## Licencia

[GPL-3.0-or-later](LICENSE).
