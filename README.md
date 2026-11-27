# ShaderCraft

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE-MIT)
[![Rust: Safe](https://img.shields.io/badge/rust-forbid%20unsafe__code-orange?logo=rust)](./Cargo.toml)
[![GUI: egui 0.37](https://img.shields.io/badge/GUI-egui%200.37-00a8e8)](./crates/shadercraft-app/Cargo.toml)
[![Export: 5-Map PBR PNG](https://img.shields.io/badge/PBR-5--map%20PNG%20export-10b981)](./crates/shadercraft-core/src/lib.rs)
[![Storage: SQLite](https://img.shields.io/badge/storage-SQLite%20bundled-003b57?logo=sqlite)](./crates/shadercraft-core/src/db.rs)
[![MCP: 2026-07-28 stdio](https://img.shields.io/badge/MCP-2026--07--28%20stdio-7c3aed)](./crates/shadercraft-mcp/src/main.rs)
[![ArtCraft: 3-Tier](https://img.shields.io/badge/ArtCraft-L1%20%7C%20L2%20%7C%20L3-16a34a)](./Cargo.toml)

**Pure-Rust Procedural Texture Forge, Deterministic Node-Graph Synthesizer & 5-Map PBR Exporter for Game Art** (`L1 shadercraft-core` + `L2 shadercraft-mcp` + `L3 shadercraft-app`).

---

## Overview

ShaderCraft replaces heavyweight node-based texture tools with a zero-dependency native Rust desktop studio (`#![forbid(unsafe_code)]`) and sandboxed MCP `2026-07-28` server. Given the same graph and seed, the deterministic CPU engine renders byte-identical tileable textures on every machine and derives a complete 5-map PBR material set (`albedo`, `height`, `normal`, `roughness`, `metallic`).

It bundles **5 integrated workbenches** in one unified workspace:

1. **Graph Editor** -- node list with per-node parameter sliders, wire list, and add/connect forms across 13 node kinds (`value_noise`, `gradient_noise`, `voronoi`, `fbm`, `linear_gradient`, `radial_gradient`, `sdf_circle`, `sdf_box`, `threshold`, `smoothstep`, `blend`, `invert`, `levels`).
2. **2D Preview** -- renders the graph output heightfield at 64, 128, or 256 pixels with live min/max/mean statistics and one-click PBR export.
3. **PBR Maps** -- all five derived maps side by side (`albedo`, `height`, `normal`, `roughness`, `metallic`) with a details view.
4. **Presets** -- four built-in game-art materials (`brushed_metal`, `mossy_stone`, `lava`, `blue_marble`).
5. **Assistant & Help** -- local-first chat grounded in live engine state and 6-tab interactive documentation.

---

## Feature Matrix

| Capability | Status | Crate / Module |
|---|---|---|
| Deterministic CPU Node-Graph Texture Engine (13 node kinds, seeded tileable noise) | `Implemented` | `shadercraft-core` |
| Fractal Brownian Motion, Voronoi F1/Edge, Gradient and Value Noise | `Implemented` | `shadercraft-core::noise` |
| Height-to-Normal Conversion and PBR Map Derivation (5 maps: albedo, height, normal, roughness, metallic) | `Implemented` | `shadercraft-core` |
| PNG Export via Bundled Encoder (`64..1024` px, single map or full 5-map PBR set) | `Implemented` | `shadercraft-core` |
| 4 Built-In Material Presets (`brushed_metal`, `mossy_stone`, `lava`, `blue_marble`) | `Implemented` | `shadercraft-core::model` |
| Bundled SQLite Persistence (`%LOCALAPPDATA%\shadercraft\shadercraft.db` or `--db <PATH>`) | `Implemented` | `shadercraft-core::db` |
| CLI (`--json`) + MCP `2026-07-28` Stdio Server (`13` tools, mandatory automation roots) | `Implemented` | `shadercraft-mcp` |
| Native `eframe`/`egui` 0.37 Studio (5 Workbenches, Dark/Light themes, `Ctrl+K`, `F1`, `Ctrl+Wheel` zoom) | `Implemented` | `shadercraft-app` |

---

## Node Library & Built-In Presets

### 13 Procedural Node Kinds

| Category | Node Kinds |
|---|---|
| **Noise Generators** | `value_noise`, `gradient_noise`, `voronoi`, `fbm` |
| **Gradients & SDF Shapes** | `linear_gradient`, `radial_gradient`, `sdf_circle`, `sdf_box` |
| **Filters & Combinators** | `threshold`, `smoothstep`, `blend`, `invert`, `levels` |

### 4 Built-In Game-Art Presets

| Preset ID | Material |
|---|---|
| `brushed_metal` | Anisotropic brushed metallic surface |
| `mossy_stone` | Organic stone heightfield with moss coloration and roughness variation |
| `lava` | Cracked volcanic crust with emissive magma veins |
| `blue_marble` | Veined marble surface with smooth specular finish |

---

## Architecture (3-Tier Workspace Seam)

```text
shadercraft-app (L3: eframe/egui 5-workbench studio) --+
                                                       +--> shadercraft-core (L1: Command::dispatch, CPU node graph, PBR, SQLite)
shadercraft-mcp (L2: clap CLI + sandboxed MCP stdio) --+
```

- **L1 (`shadercraft-core`)**: Pure-Rust deterministic procedural node-graph evaluator, seeded tileable noise library, Sobel/height-to-normal PBR map derivation, PNG encoder, bundled `rusqlite` persistence, and `Command::dispatch` (zero UI dependencies).
- **L2 (`shadercraft-mcp`)**: CLI-first `clap` binary (`--json`, `--db <PATH>`) and JSON-RPC `MCP 2026-07-28` stdio server exposing 13 tools. **Sandbox enforcement**: `--mcp` refuses to start without both `--automation-read-root <DIR>` and `--automation-write-root <DIR>`, and every file write is canonicalized and confined inside the write root.
- **L3 (`shadercraft-app`)**: Native `eframe`/`egui` 0.37 desktop studio calling `shadercraft-core` directly in-process across all 5 workbenches.

---

## CLI & MCP Reference

### CLI Subcommands & MCP Tools (`13` tools)

| CLI Subcommand | MCP Tool Name | Description |
|---|---|---|
| `status` | `shader_status` | Show engine status and active graph info |
| `summary` | `shader_summary` | Full active graph summary with nodes and wires |
| `create-graph --name N` | `create_graph` | Create a new texture graph seeded with a starter chain |
| `list-graphs` | `list_graphs` | List all persisted texture graphs |
| `list-presets` | `list_presets` | List the 4 built-in material presets |
| `load-preset --preset P` | `load_preset` | Load a preset (`brushed_metal`, `mossy_stone`, `lava`, `blue_marble`) as a new graph |
| `add-node --kind K [--param k=v]` | `add_node` | Add a node with repeatable `key=value` parameters |
| `set-param --node-id ID --key K --value V` | `set_param` | Set one node parameter (or output key with `--node-id output`) |
| `connect-nodes --from A --to B [--input 0]` | `connect_nodes` | Wire a node output into a target node input slot |
| `delete-node --node-id ID` | `delete_node` | Delete a node and its attached wires |
| `eval-preview-stats [--size 256]` | `eval_preview_stats` | Evaluate heightfield min/max/mean statistics |
| `render-png --map M --size S --out FILE` | `render_png` | Render one map (`albedo`, `height`, `normal`, `roughness`, `metallic`) to PNG |
| `export-pbr-set --size S --out-dir DIR` | `export_pbr_set` | Export all 5 PBR maps as PNGs into a directory |

---

## Quick Start

```powershell
# Launch desktop studio (or double-click start.bat)
.\start.ps1

# Run headless smoke test
.\start.ps1 -Smoke

# Run automated verification gates
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

# Run CLI commands
cargo run -p shadercraft-mcp -- status --json
cargo run -p shadercraft-mcp -- load-preset --preset lava
cargo run -p shadercraft-mcp -- eval-preview-stats --size 128
cargo run -p shadercraft-mcp -- render-png --map normal --size 256 --out lava_normal.png
cargo run -p shadercraft-mcp -- export-pbr-set --size 256 --out-dir lava_set

# Run MCP 2026-07-28 stdio server (both roots required)
cargo run -p shadercraft-mcp -- --mcp --automation-read-root C:\art\in --automation-write-root C:\art\out
```

### Keyboard Shortcuts & Tips

- **Command Palette**: `Ctrl + K`
- **In-App Help**: `F1`
- **UI Zoom**: `Ctrl + Mouse Wheel`
- Keep `tileable` enabled on noise nodes for repeating game-engine textures.
- A flat heightfield renders neutral normal color `(128, 128, 255)`: if your normal map is solid blue-purple, the graph output is constant.

---

## Documentation

- [HELP.md](./HELP.md) -- Full user guide, CLI/MCP examples, and tips
- [CHANGELOG.md](./CHANGELOG.md) -- Version history
- [ATTRIBUTION.md](./ATTRIBUTION.md) & [NOTICE](./NOTICE) -- Third-party credits and notices

## Lineage & Licensing

ShaderCraft follows the Storytold ArtCraft 3-tier craft-app pattern ([storytold/artcraft](https://github.com/storytold/artcraft)): headless-first native Rust binaries sharing one L1 `Command::dispatch` bus across GUI, CLI, and MCP stdio.

Dual-licensed under `MIT OR Apache-2.0`. See [LICENSE-MIT](./LICENSE-MIT), [LICENSE-APACHE](./LICENSE-APACHE), [NOTICE](./NOTICE), and [ATTRIBUTION.md](./ATTRIBUTION.md).
