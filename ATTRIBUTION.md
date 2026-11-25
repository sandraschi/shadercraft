# Attribution & Third-Party Notices

ShaderCraft is dual-licensed under `MIT OR Apache-2.0` and authored by Sandra Schipal (`sandraschi`).

## Architectural Lineage

- **Storytold ArtCraft Pattern** (`storytold/artcraft` by Brandon Thomas): 3-tier Pure-Rust architecture (`L1 core` -> `L2 CLI + MCP stdio` -> `L3 eframe/egui desktop studio`). All code in ShaderCraft is original and independently written.

## Rust Crates

- `egui` / `eframe` / `egui_kittest` 0.37 -- Emil Ernerfeldt & Embark Studios (`MIT OR Apache-2.0`)
- `rusqlite` 0.41 (`bundled` SQLite) -- The rusqlite Developers (`MIT`)
- `clap` 4.5 -- clap Contributors (`MIT OR Apache-2.0`)
- `serde` / `serde_json` 1.0 -- Erick Tryzelaar & David Tolnay (`MIT OR Apache-2.0`)
- `thiserror` 2.0 -- David Tolnay (`MIT OR Apache-2.0`)
- `image` 0.25 (PNG encoder only, default features off) -- PistonDevelopers / image-rs Contributors (`MIT OR Apache-2.0`)

## Bundled Assets

- `assets/shader.svg` -- Original vector texture-node icon authored for ShaderCraft (`CC0-1.0 / MIT`).
