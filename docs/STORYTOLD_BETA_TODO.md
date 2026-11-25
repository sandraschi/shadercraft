# Storytold ArtCraft Beta Submission Checklist -- ShaderCraft

- [x] 3-tier Pure-Rust workspace (`shadercraft-core`, `shadercraft-mcp`, `shadercraft-app`)
- [x] `#![forbid(unsafe_code)]` and `clippy::deny(unwrap_used, expect_used, panic, todo, unimplemented)`
- [x] 5 integrated workbenches in hierarchical `SubApp` sidebar
- [x] Bundled SQLite persistence (`rusqlite` 0.41)
- [x] 12 MCP `2026-07-28` stdio tools + CLI subcommands sharing `Command::dispatch`
- [x] Headless verification across every sub-app workbench
- [ ] Attach Dark/Light studio screenshots before public submission to `https://getartcraft.com/beta`
