# ShaderCraft

**Pure-Rust Procedural Texture Forge, Node-Graph Synthesizer with PBR Map Export for Game Art** (`L1 shadercraft-core` + `L2 shadercraft-mcp` + `L3 shadercraft-app`).

## Overview

ShaderCraft replaces heavyweight node-based texture tools with a zero-dependency native Rust desktop studio and MCP `2026-07-28` server. It bundles 5 hierarchical workbenches in one unified workspace:

1. **Graph Editor** -- node list with per-node parameter sliders, wire list, and add/connect forms (`value_noise`, `gradient_noise`, `voronoi`, `fbm`, `linear_gradient`, `radial_gradient`, `sdf_circle`, `sdf_box`, `threshold`, `smoothstep`, `blend`, `invert`, `levels`).
2. **2D Preview** -- renders the graph output heightfield at 64, 128, or 256 pixels with live statistics and one-click PBR export.
3. **PBR Maps** -- all five derived maps side by side (`albedo`, `height`, `normal`, `roughness`, `metallic`) with a details view.
4. **Presets** -- four built-in game-art materials (`brushed_metal`, `mossy_stone`, `lava`, `blue_marble`).
5. **Assistant & Help** -- local-first chat grounded in live engine state and 6-tab interactive documentation.

## Feature Matrix

| Capability | Status | Crate / Module |
|---|---|---|
| Deterministic CPU Node-Graph Texture Engine (13 node kinds, seeded noise) | `Implemented` | `shadercraft-core` |
| Fractal Brownian Motion, Voronoi F1/Edge, Gradient and Value Noise | `Implemented` | `shadercraft-core::noise` |
| Height-to-Normal Conversion and PBR Map Derivation (5 maps) | `Implemented` | `shadercraft-core` |
| PNG Export via Bundled Encoder (64..1024 px) | `Implemented` | `shadercraft-core` |
| 4 Built-In Material Presets (Metal, Stone, Lava, Marble) | `Implemented` | `shadercraft-core::model` |
| Bundled SQLite Persistence (`%LOCALAPPDATA%\shadercraft\shadercraft.db`) | `Implemented` | `shadercraft-core::db` |
| CLI (`--json`) + MCP `2026-07-28` Stdio Server (`13` tools, automation roots) | `Implemented` | `shadercraft-mcp` |
| Native `eframe`/`egui` 0.37 Studio (5 Workbenches, Dark/Light) | `Implemented` | `shadercraft-app` |

## Quick Start

```powershell
# Launch desktop studio
.\start.ps1

# Run headless smoke test
.\start.ps1 -Smoke

# Run CLI commands
cargo run -p shadercraft-mcp -- status --json
cargo run -p shadercraft-mcp -- load-preset --preset lava
cargo run -p shadercraft-mcp -- eval-preview-stats --size 128
cargo run -p shadercraft-mcp -- render-png --map normal --size 256 --out lava_normal.png

# Run MCP 2026-07-28 stdio server (both roots required)
cargo run -p shadercraft-mcp -- --mcp --automation-read-root C:\art\in --automation-write-root C:\art\out
```

## License

Dual-licensed under `MIT OR Apache-2.0`.
