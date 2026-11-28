# Changelog

## [Unreleased]

### Added
- Wired shared `craft-ai-ui` (`LlmSettingsPanel` + async `complete_live`) into `shadercraft-app` (`Assistant` tab, `Window > LLM Settings...`, and `Ctrl+K` Command Palette) backed by `~/.craft/ai_config.json` and grounded in live texture engine state.

## [0.1.0] - 2026-11-24

### Added
- Built 3-tier Pure-Rust ArtCraft workspace (`shadercraft-core`, `shadercraft-mcp`, `shadercraft-app`).
- Added deterministic CPU node-graph texture engine (13 node kinds, seeded value/gradient/Voronoi/fBm noise with seamless tiling), height-to-normal conversion, and five-map PBR derivation.
- Added 4 built-in material presets (`brushed_metal`, `mossy_stone`, `lava`, `blue_marble`) and PNG export (64..1024 px).
- Added 13 MCP `2026-07-28` tools and matching CLI subcommands sharing `Command::dispatch`, with automation-root enforcement on every file write.
- Added 5 studio workbenches (`Graph Editor`, `2D Preview`, `PBR Maps`, `Presets`, `Assistant & Help`) with bundled SQLite persistence (`rusqlite` 0.41).
