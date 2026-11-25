# ShaderCraft Help & User Guide

## Get Started

**What is ShaderCraft?**
ShaderCraft is a 3-tier Pure-Rust ArtCraft procedural texture forge: a deterministic CPU node-graph texture synthesizer, a 2D tile previewer with live statistics, a five-map PBR derivation workbench, and a PNG export forge for game art.

It provides 5 integrated sub-app workbenches in the left sidebar:
1. **Graph Editor** -- node list with per-node parameter sliders (`value_noise`, `gradient_noise`, `voronoi`, `fbm`, gradients, SDF shapes, filters, blend) plus a wire list with add/connect forms.
2. **2D Preview** -- renders the active graph at 64, 128, or 256 pixels with min/max/mean statistics.
3. **PBR Maps** -- all five derived maps side by side (`albedo`, `height`, `normal`, `roughness`, `metallic`).
4. **Presets** -- four built-in game-art materials: brushed metal, mossy stone, lava, and blue marble.
5. **Assistant & Help** -- local-first chat grounded in engine state and 6-tab interactive documentation.

## Functions

- **Texture Presets**: `brushed_metal`, `mossy_stone`, `lava`, `blue_marble`.
- **SQLite Persistence**: Auto-saves graphs to `%LOCALAPPDATA%\shadercraft\shadercraft.db` (or `--db <PATH>`).
- **Deterministic Seeds**: The same graph with the same seed renders byte-identical tiles on every machine.

## Usage

### CLI
```powershell
shadercraft-mcp status --json
shadercraft-mcp load-preset --preset lava
shadercraft-mcp eval-preview-stats --size 128
shadercraft-mcp render-png --map albedo --size 256 --out lava_albedo.png
shadercraft-mcp export-pbr-set --size 256 --out-dir lava_set
```

### MCP Server (`2026-07-28` stdio)
```powershell
shadercraft-mcp --mcp --automation-read-root C:\art\in --automation-write-root C:\art\out
```
Exposes 13 MCP tools: `shader_status`, `shader_summary`, `create_graph`, `list_graphs`, `list_presets`, `load_preset`, `add_node`, `set_param`, `connect_nodes`, `delete_node`, `eval_preview_stats`, `render_png`, and `export_pbr_set`. Both automation roots are required at startup, and every file write stays inside the write root.

## Tips

- Tune noise seeds in small steps: seed `7` versus seed `8` gives a related but visibly different weave.
- Keep `tileable` on for noise nodes when the texture must repeat in a game engine.
- Press `Ctrl+K` for the Command Palette, `F1` for Help, and `Ctrl+MouseWheel` to zoom the UI.
- A flat heightfield renders the neutral normal color (128, 128, 255): if your normal map is all blue-purple, the graph output is constant.

## Other Crafts

ShaderCraft is part of the `sandraschi` Pure-Rust ArtCraft ecosystem alongside `logiccraft`, `graphcraft`, `mathcraft`, `robotcraft`, `smbcraft`, and `gamecraft`.
