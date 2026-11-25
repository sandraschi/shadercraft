//! ShaderCraft L2 CLI & MCP `2026-07-28` stdio server.
//!
//! Agent entry points (`shadercraft-mcp --mcp`) refuse to start without both
//! `--automation-read-root` and `--automation-write-root`, and refuse every
//! file access outside those roots (rule 18).
#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use shadercraft_core::{default_db_path, Command, ShaderEngine, APP_VERSION};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// Resolved automation roots for one MCP server session.
#[derive(Debug, Clone)]
pub struct AutomationRoots {
    pub read: PathBuf,
    pub write: PathBuf,
}

fn take_flag(argv: &[String], flag: &str) -> Result<Option<PathBuf>, String> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        let arg = &argv[i];
        if arg == flag {
            i += 1;
            if i >= argv.len() {
                return Err(format!("{flag} needs a directory ({flag} <dir>)"));
            }
            found.push(PathBuf::from(&argv[i]));
        } else if let Some(rest) = arg.strip_prefix(flag) {
            if let Some(value) = rest.strip_prefix('=') {
                found.push(PathBuf::from(value));
            }
        }
        i += 1;
    }
    if found.len() > 1 {
        return Err(format!(
            "{flag} was given {} times; pass exactly one directory",
            found.len()
        ));
    }
    Ok(found.into_iter().next())
}

/// Parse and validate both automation roots. Every failure refuses startup
/// before any tool runs.
pub fn parse_automation_roots(argv: &[String]) -> Result<AutomationRoots, String> {
    let read = take_flag(argv, "--automation-read-root")?
        .ok_or_else(|| "refusing to start: missing --automation-read-root <dir>".to_string())?;
    let write = take_flag(argv, "--automation-write-root")?
        .ok_or_else(|| "refusing to start: missing --automation-write-root <dir>".to_string())?;
    for (label, path) in [("read", &read), ("write", &write)] {
        if !path.is_dir() {
            return Err(format!(
                "refusing to start: --automation-{label}-root is not an existing directory: {}",
                path.display()
            ));
        }
    }
    let read = read
        .canonicalize()
        .map_err(|e| format!("refusing to start: cannot resolve read root: {e}"))?;
    let write = write
        .canonicalize()
        .map_err(|e| format!("refusing to start: cannot resolve write root: {e}"))?;
    Ok(AutomationRoots { read, write })
}

/// Resolve a tool-supplied path and refuse it unless it stays inside the
/// write root. Links are followed from the nearest existing folder, and the
/// comparison is by whole folder names.
pub fn enforce_write_path(
    roots: &AutomationRoots,
    cwd: &Path,
    raw: &str,
) -> Result<PathBuf, String> {
    let requested = Path::new(raw);
    let abs = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        cwd.join(requested)
    };
    let mut anchor = abs.clone();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    while !anchor.exists() {
        let parent = anchor.parent().map(Path::to_path_buf);
        let name = anchor.file_name().map(|s| s.to_os_string());
        let (Some(p), Some(n)) = (parent, name) else {
            break;
        };
        tail.push(n);
        anchor = p;
        if tail.len() > 512 {
            return Err(format!("refusing path with too many components: {raw}"));
        }
    }
    if !anchor.exists() {
        return Err(format!("refusing path with no existing folder: {raw}"));
    }
    let canon = anchor
        .canonicalize()
        .map_err(|e| format!("cannot resolve '{}': {e}", anchor.display()))?;
    let mut full = canon;
    for comp in tail.iter().rev() {
        full.push(comp);
    }
    if full != roots.write && !full.starts_with(&roots.write) {
        return Err(format!(
            "refusing to write outside the automation write root ({}): {raw}",
            roots.write.display()
        ));
    }
    Ok(full)
}

#[derive(Debug, Parser)]
#[command(
    name = "shadercraft-mcp",
    version,
    about = "ShaderCraft Procedural Texture Forge, Node-Graph Synthesizer & PBR Map Exporter"
)]
pub struct Cli {
    /// Emit compact single-line JSON output
    #[arg(long, global = true, default_value_t = false)]
    pub json: bool,

    /// Optional SQLite database file path override
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

    /// Launch MCP 2026-07-28 JSON-RPC server on stdio
    #[arg(long, global = true, default_value_t = false)]
    pub mcp: bool,

    /// Automation read root (required for --mcp, enforced for CLI writes)
    #[arg(long, global = true)]
    pub automation_read_root: Option<PathBuf>,

    /// Automation write root (required for --mcp, enforced for CLI writes)
    #[arg(long, global = true)]
    pub automation_write_root: Option<PathBuf>,

    #[command(subcommand)]
    pub subcommand: Option<CliSub>,
}

#[derive(Debug, Subcommand)]
pub enum CliSub {
    /// Run MCP 2026-07-28 JSON-RPC server on stdio
    Mcp,
    /// Show ShaderCraft engine status
    Status,
    /// Show full engine summary (active graph with nodes and edges)
    Summary,
    /// Create a new texture graph seeded with a starter chain
    CreateGraph {
        #[arg(long)]
        name: String,
    },
    /// List all graphs
    ListGraphs,
    /// List the four built-in material presets
    ListPresets,
    /// Load a built-in preset as a new graph (brushed_metal, mossy_stone, lava, blue_marble)
    LoadPreset {
        #[arg(long, default_value = "brushed_metal")]
        preset: String,
    },
    /// Add a node to a graph (--param key=value, repeatable)
    AddNode {
        #[arg(long)]
        graph_id: Option<String>,
        #[arg(long)]
        kind: String,
        #[arg(long = "param")]
        params: Vec<String>,
    },
    /// Set one node parameter (or graph output key with node id "output")
    SetParam {
        #[arg(long)]
        graph_id: Option<String>,
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        value: f32,
    },
    /// Connect a texture output into a node input slot
    ConnectNodes {
        #[arg(long)]
        graph_id: Option<String>,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long, default_value_t = 0)]
        input: u32,
    },
    /// Delete a node and its attached wires
    DeleteNode {
        #[arg(long)]
        graph_id: Option<String>,
        #[arg(long)]
        node_id: String,
    },
    /// Evaluate heightfield statistics at a render size
    EvalPreviewStats {
        #[arg(long)]
        graph_id: Option<String>,
        #[arg(long, default_value_t = 256)]
        size: u32,
    },
    /// Render one map to a PNG file (albedo, height, normal, roughness, metallic)
    RenderPng {
        #[arg(long)]
        graph_id: Option<String>,
        #[arg(long, default_value_t = 256)]
        size: u32,
        #[arg(long, default_value = "albedo")]
        map: String,
        #[arg(long)]
        out: String,
    },
    /// Export the full five-map PBR set into a directory
    ExportPbrSet {
        #[arg(long)]
        graph_id: Option<String>,
        #[arg(long, default_value_t = 256)]
        size: u32,
        #[arg(long)]
        out_dir: String,
    },
}

fn active_or(graph_id: &Option<String>, engine: &ShaderEngine) -> String {
    graph_id
        .clone()
        .unwrap_or_else(|| engine.active_graph.clone())
}

fn parse_cli_params(raw: &[String]) -> Result<HashMap<String, f32>, String> {
    let mut map = HashMap::new();
    for item in raw {
        let (key, value) = item.split_once('=').ok_or_else(|| {
            format!("--param '{item}' must have key=value form (example: --param scale=8)")
        })?;
        let parsed: f32 = value
            .parse()
            .map_err(|_| format!("--param '{item}': '{value}' is not a number"))?;
        map.insert(key.to_string(), parsed);
    }
    Ok(map)
}

fn cli_roots(cli: &Cli) -> Result<Option<AutomationRoots>, String> {
    match (&cli.automation_read_root, &cli.automation_write_root) {
        (None, None) => Ok(None),
        (Some(read), Some(write)) => {
            for (label, path) in [("read", read), ("write", write)] {
                if !path.is_dir() {
                    return Err(format!(
                        "--automation-{label}-root is not an existing directory: {}",
                        path.display()
                    ));
                }
            }
            let read = read
                .canonicalize()
                .map_err(|e| format!("cannot resolve read root: {e}"))?;
            let write = write
                .canonicalize()
                .map_err(|e| format!("cannot resolve write root: {e}"))?;
            Ok(Some(AutomationRoots { read, write }))
        }
        _ => Err(
            "pass both --automation-read-root and --automation-write-root, or neither".to_string(),
        ),
    }
}

fn print_value(cli: &Cli, val: &Value) {
    if cli.json {
        println!(
            "{}",
            serde_json::to_string(val).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(val).unwrap_or_else(|_| "{}".to_string())
        );
    }
}

fn main() {
    let cli = Cli::parse();
    let db_path = cli.db.clone().unwrap_or_else(default_db_path);
    let mut engine = ShaderEngine::open_or_seed(&db_path).unwrap_or_default();

    if cli.mcp || matches!(cli.subcommand, Some(CliSub::Mcp)) {
        let argv: Vec<String> = std::env::args().collect();
        match parse_automation_roots(&argv) {
            Ok(roots) => run_mcp_stdio(&mut engine, &db_path, &roots),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(2);
            }
        }
        return;
    }

    let roots = match cli_roots(&cli) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let cmd = match &cli.subcommand {
        None | Some(CliSub::Mcp) | Some(CliSub::Status) => Command::Status,
        Some(CliSub::Summary) => Command::Summary,
        Some(CliSub::CreateGraph { name }) => Command::CreateGraph { name: name.clone() },
        Some(CliSub::ListGraphs) => Command::ListGraphs,
        Some(CliSub::ListPresets) => Command::ListPresets,
        Some(CliSub::LoadPreset { preset }) => Command::LoadPreset {
            preset: preset.clone(),
        },
        Some(CliSub::AddNode {
            graph_id,
            kind,
            params,
        }) => {
            let parsed = match parse_cli_params(params) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            };
            Command::AddNode {
                graph_id: active_or(graph_id, &engine),
                kind: kind.clone(),
                params: parsed,
            }
        }
        Some(CliSub::SetParam {
            graph_id,
            node_id,
            key,
            value,
        }) => Command::SetParam {
            graph_id: active_or(graph_id, &engine),
            node_id: node_id.clone(),
            key: key.clone(),
            value: *value,
        },
        Some(CliSub::ConnectNodes {
            graph_id,
            from,
            to,
            input,
        }) => Command::ConnectNodes {
            graph_id: active_or(graph_id, &engine),
            from: from.clone(),
            to: to.clone(),
            input: *input,
        },
        Some(CliSub::DeleteNode { graph_id, node_id }) => Command::DeleteNode {
            graph_id: active_or(graph_id, &engine),
            node_id: node_id.clone(),
        },
        Some(CliSub::EvalPreviewStats { graph_id, size }) => Command::EvalPreviewStats {
            graph_id: active_or(graph_id, &engine),
            size: *size,
        },
        Some(CliSub::RenderPng {
            graph_id,
            size,
            map,
            out,
        }) => {
            let gid = active_or(graph_id, &engine);
            let target = match &roots {
                Some(r) => match enforce_write_path(r, &cwd, out) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("error: {e}");
                        std::process::exit(2);
                    }
                },
                None => PathBuf::from(out),
            };
            match engine.render_png_bytes(&gid, *size, map) {
                Ok(bytes) => {
                    if let Some(parent) = target.parent() {
                        if !parent.as_os_str().is_empty()
                            && std::fs::create_dir_all(parent).is_err()
                        {
                            eprintln!("error: cannot create output directory");
                            std::process::exit(1);
                        }
                    }
                    if std::fs::write(&target, &bytes).is_err() {
                        eprintln!("error: cannot write {}", target.display());
                        std::process::exit(1);
                    }
                    let _ = engine.save_db(&db_path);
                    print_value(
                        &cli,
                        &json!({
                            "graph_id": gid,
                            "map": map,
                            "written": target.display().to_string(),
                            "png_bytes": bytes.len(),
                        }),
                    );
                    return;
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(CliSub::ExportPbrSet {
            graph_id,
            size,
            out_dir,
        }) => {
            let gid = active_or(graph_id, &engine);
            let target = match &roots {
                Some(r) => match enforce_write_path(r, &cwd, out_dir) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("error: {e}");
                        std::process::exit(2);
                    }
                },
                None => PathBuf::from(out_dir),
            };
            match engine.export_pbr_set(&gid, *size) {
                Ok(files) => {
                    if std::fs::create_dir_all(&target).is_err() {
                        eprintln!("error: cannot create {}", target.display());
                        std::process::exit(1);
                    }
                    let mut written = Vec::new();
                    for (name, bytes) in &files {
                        let dest = target.join(name);
                        if std::fs::write(&dest, bytes).is_err() {
                            eprintln!("error: cannot write {}", dest.display());
                            std::process::exit(1);
                        }
                        written.push(dest.display().to_string());
                    }
                    let _ = engine.save_db(&db_path);
                    print_value(&cli, &json!({ "graph_id": gid, "written": written }));
                    return;
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }
    };

    match cmd.dispatch(&mut engine) {
        Ok(val) => {
            let _ = engine.save_db(&db_path);
            print_value(&cli, &val);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

fn tool_graph_id(args: &Value, engine: &ShaderEngine) -> String {
    args.get("graph_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| engine.active_graph.clone())
}

pub fn handle_mcp_request(
    req: &Value,
    engine: &mut ShaderEngine,
    db_path: &Path,
    roots: &AutomationRoots,
) -> Option<Value> {
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let id = req.get("id").cloned();

    if method.starts_with("notifications/") {
        return None;
    }
    let req_id = id?;

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "protocolVersion": "2026-07-28",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": {
                    "name": "shadercraft-mcp",
                    "version": APP_VERSION
                }
            }
        })),
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "tools": mcp_tools_schema()
            }
        })),
        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or_else(|| json!({}));
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

            let cmd_res: Result<Value, shadercraft_core::ShaderError> = match name {
                "shader_status" => Command::Status.dispatch(engine),
                "shader_summary" => Command::Summary.dispatch(engine),
                "create_graph" => {
                    let graph_name = args
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    Command::CreateGraph { name: graph_name }.dispatch(engine)
                }
                "list_graphs" => Command::ListGraphs.dispatch(engine),
                "list_presets" => Command::ListPresets.dispatch(engine),
                "load_preset" => {
                    let preset = args
                        .get("preset")
                        .and_then(Value::as_str)
                        .unwrap_or("brushed_metal")
                        .to_string();
                    Command::LoadPreset { preset }.dispatch(engine)
                }
                "add_node" => {
                    let kind = args
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let mut pmap = HashMap::new();
                    if let Some(obj) = args.get("params").and_then(Value::as_object) {
                        for (k, v) in obj {
                            if let Some(f) = v.as_f64() {
                                pmap.insert(k.clone(), f as f32);
                            }
                        }
                    }
                    Command::AddNode {
                        graph_id: tool_graph_id(&args, engine),
                        kind,
                        params: pmap,
                    }
                    .dispatch(engine)
                }
                "set_param" => {
                    let node_id = args
                        .get("node_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let key = args
                        .get("key")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let value = args.get("value").and_then(Value::as_f64).unwrap_or(0.0) as f32;
                    Command::SetParam {
                        graph_id: tool_graph_id(&args, engine),
                        node_id,
                        key,
                        value,
                    }
                    .dispatch(engine)
                }
                "connect_nodes" => {
                    let from = args
                        .get("from")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let to = args
                        .get("to")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let input = args.get("input").and_then(Value::as_u64).unwrap_or(0) as u32;
                    Command::ConnectNodes {
                        graph_id: tool_graph_id(&args, engine),
                        from,
                        to,
                        input,
                    }
                    .dispatch(engine)
                }
                "delete_node" => {
                    let node_id = args
                        .get("node_id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    Command::DeleteNode {
                        graph_id: tool_graph_id(&args, engine),
                        node_id,
                    }
                    .dispatch(engine)
                }
                "eval_preview_stats" => {
                    let size = args.get("size").and_then(Value::as_u64).unwrap_or(256) as u32;
                    Command::EvalPreviewStats {
                        graph_id: tool_graph_id(&args, engine),
                        size,
                    }
                    .dispatch(engine)
                }
                "render_png" => {
                    let size = args.get("size").and_then(Value::as_u64).unwrap_or(256) as u32;
                    let map = args
                        .get("map")
                        .and_then(Value::as_str)
                        .unwrap_or("albedo")
                        .to_string();
                    let filename = args.get("filename").and_then(Value::as_str).unwrap_or("");
                    let gid = tool_graph_id(&args, engine);
                    let target = match enforce_write_path(roots, &cwd, filename) {
                        Ok(p) => p,
                        Err(e) => {
                            return Some(json!({
                                "jsonrpc": "2.0",
                                "id": req_id,
                                "result": {
                                    "content": [{ "type": "text", "text": e }],
                                    "isError": true
                                }
                            }));
                        }
                    };
                    match engine.render_png_bytes(&gid, size, &map) {
                        Ok(bytes) => {
                            if let Some(parent) = target.parent() {
                                if !parent.as_os_str().is_empty()
                                    && std::fs::create_dir_all(parent).is_err()
                                {
                                    return Some(json!({
                                        "jsonrpc": "2.0",
                                        "id": req_id,
                                        "result": {
                                            "content": [{ "type": "text", "text": "cannot create output directory" }],
                                            "isError": true
                                        }
                                    }));
                                }
                            }
                            match std::fs::write(&target, &bytes) {
                                Ok(()) => Ok(json!({
                                    "graph_id": gid,
                                    "map": map,
                                    "size": size,
                                    "written": target.display().to_string(),
                                    "png_bytes": bytes.len(),
                                })),
                                Err(e) => Err(shadercraft_core::ShaderError::Storage(format!(
                                    "cannot write {}: {e}",
                                    target.display()
                                ))),
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
                "export_pbr_set" => {
                    let size = args.get("size").and_then(Value::as_u64).unwrap_or(256) as u32;
                    let dir = args.get("dir").and_then(Value::as_str).unwrap_or("pbr");
                    let gid = tool_graph_id(&args, engine);
                    let target = match enforce_write_path(roots, &cwd, dir) {
                        Ok(p) => p,
                        Err(e) => {
                            return Some(json!({
                                "jsonrpc": "2.0",
                                "id": req_id,
                                "result": {
                                    "content": [{ "type": "text", "text": e }],
                                    "isError": true
                                }
                            }));
                        }
                    };
                    match engine.export_pbr_set(&gid, size) {
                        Ok(files) => {
                            if std::fs::create_dir_all(&target).is_err() {
                                return Some(json!({
                                    "jsonrpc": "2.0",
                                    "id": req_id,
                                    "result": {
                                        "content": [{ "type": "text", "text": "cannot create export directory" }],
                                        "isError": true
                                    }
                                }));
                            }
                            let mut written = Vec::new();
                            for (fname, bytes) in &files {
                                let dest = target.join(fname);
                                if std::fs::write(&dest, bytes).is_err() {
                                    return Some(json!({
                                        "jsonrpc": "2.0",
                                        "id": req_id,
                                        "result": {
                                            "content": [{ "type": "text", "text": format!("cannot write {}", dest.display()) }],
                                            "isError": true
                                        }
                                    }));
                                }
                                written.push(dest.display().to_string());
                            }
                            Ok(json!({
                                "graph_id": gid,
                                "size": size,
                                "dir": target.display().to_string(),
                                "written": written,
                            }))
                        }
                        Err(e) => Err(e),
                    }
                }
                other => Err(shadercraft_core::ShaderError::Validation(format!(
                    "Unknown tool '{other}'"
                ))),
            };

            match cmd_res {
                Ok(val) => {
                    let _ = engine.save_db(db_path);
                    let text = serde_json::to_string_pretty(&val).unwrap_or_default();
                    Some(json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "content": [{ "type": "text", "text": text }],
                            "isError": false
                        }
                    }))
                }
                Err(e) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [{ "type": "text", "text": e.to_string() }],
                        "isError": true
                    }
                })),
            }
        }
        _ => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "error": { "code": -32601, "message": format!("Method not found: {method}") }
        })),
    }
}

fn mcp_tools_schema() -> Value {
    json!([
        { "name": "shader_status", "description": "Get ShaderCraft engine status, active graph, and node/edge counts.", "inputSchema": { "type": "object", "properties": {} } },
        { "name": "shader_summary", "description": "Return the full active texture graph (nodes, edges, output, color ramp).", "inputSchema": { "type": "object", "properties": {} } },
        { "name": "create_graph", "description": "Create a new texture graph seeded with a starter chain.", "inputSchema": { "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"] } },
        { "name": "list_graphs", "description": "List all texture graphs with node and edge counts.", "inputSchema": { "type": "object", "properties": {} } },
        { "name": "list_presets", "description": "List the four built-in material presets.", "inputSchema": { "type": "object", "properties": {} } },
        { "name": "load_preset", "description": "Load a built-in preset as a new graph (brushed_metal, mossy_stone, lava, blue_marble).", "inputSchema": { "type": "object", "properties": { "preset": { "type": "string" } }, "required": ["preset"] } },
        { "name": "add_node", "description": "Add a texture node (value_noise, gradient_noise, voronoi, fbm, linear_gradient, radial_gradient, sdf_circle, sdf_box, threshold, smoothstep, blend, invert, levels).", "inputSchema": { "type": "object", "properties": { "graph_id": { "type": "string" }, "kind": { "type": "string" }, "params": { "type": "object" } }, "required": ["kind"] } },
        { "name": "set_param", "description": "Set one node parameter, or a graph output key (node_id 'output': normal_strength, roughness_base, roughness_scale, metallic_base, metallic_scale).", "inputSchema": { "type": "object", "properties": { "graph_id": { "type": "string" }, "node_id": { "type": "string" }, "key": { "type": "string" }, "value": { "type": "number" } }, "required": ["node_id", "key", "value"] } },
        { "name": "connect_nodes", "description": "Connect a texture output into a node input slot.", "inputSchema": { "type": "object", "properties": { "graph_id": { "type": "string" }, "from": { "type": "string" }, "to": { "type": "string" }, "input": { "type": "integer" } }, "required": ["from", "to"] } },
        { "name": "delete_node", "description": "Delete a node and its attached wires.", "inputSchema": { "type": "object", "properties": { "graph_id": { "type": "string" }, "node_id": { "type": "string" } }, "required": ["node_id"] } },
        { "name": "eval_preview_stats", "description": "Evaluate heightfield min, max, and mean at a render size.", "inputSchema": { "type": "object", "properties": { "graph_id": { "type": "string" }, "size": { "type": "integer" } } } },
        { "name": "render_png", "description": "Render one PBR map (albedo, height, normal, roughness, metallic) to a PNG file inside the write root.", "inputSchema": { "type": "object", "properties": { "graph_id": { "type": "string" }, "size": { "type": "integer" }, "map": { "type": "string" }, "filename": { "type": "string" } }, "required": ["filename"] } },
        { "name": "export_pbr_set", "description": "Export the full five-map PBR set into a directory inside the write root.", "inputSchema": { "type": "object", "properties": { "graph_id": { "type": "string" }, "size": { "type": "integer" }, "dir": { "type": "string" } } } }
    ])
}

fn run_mcp_stdio(engine: &mut ShaderEngine, db_path: &Path, roots: &AutomationRoots) {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(raw) = line else { break };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        if let Some(resp) = handle_mcp_request(&req, engine, db_path, roots) {
            if let Ok(s) = serde_json::to_string(&resp) {
                let _ = writeln!(stdout, "{s}");
                let _ = stdout.flush();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "shadercraft_mcp_test_{name}_{}.db",
            std::process::id()
        ));
        p
    }

    fn temp_roots(name: &str) -> (AutomationRoots, PathBuf) {
        let mut base = std::env::temp_dir();
        base.push(format!("shadercraft_roots_{name}_{}", std::process::id()));
        let read = base.join("read");
        let write = base.join("write");
        let _ = std::fs::create_dir_all(&read);
        let _ = std::fs::create_dir_all(&write);
        let roots = AutomationRoots {
            read: read.canonicalize().unwrap_or(read),
            write: write.canonicalize().unwrap_or(write),
        };
        (roots, base)
    }

    #[test]
    fn automation_roots_require_both_existing_directories() {
        assert!(parse_automation_roots(&[]).is_err());
        assert!(parse_automation_roots(&[
            "--automation-read-root".to_string(),
            "C:\\".to_string()
        ])
        .is_err());
        let missing = temp_db("nope").join("missing");
        let argv = vec![
            "shadercraft-mcp".to_string(),
            "--mcp".to_string(),
            "--automation-read-root".to_string(),
            missing.display().to_string(),
        ];
        assert!(parse_automation_roots(&argv).is_err());
        let dup = vec![
            "shadercraft-mcp".to_string(),
            "--automation-read-root".to_string(),
            "C:\\".to_string(),
            "--automation-read-root".to_string(),
            "C:\\".to_string(),
            "--automation-write-root".to_string(),
            "C:\\".to_string(),
        ];
        assert!(parse_automation_roots(&dup).is_err());
    }

    #[test]
    fn write_outside_the_write_root_is_refused() {
        let (roots, _base) = temp_roots("refuse");
        let cwd = std::env::temp_dir();
        assert!(enforce_write_path(&roots, &cwd, "C:\\Windows\\evil.png").is_err());
        let ok = enforce_write_path(&roots, &roots.write, "tile.png");
        assert!(ok.is_ok());
        if let Ok(p) = ok {
            assert!(p.starts_with(&roots.write));
        }
        let _ = std::fs::remove_dir_all(_base);
    }

    #[test]
    fn parent_traversal_cannot_escape_the_write_root() {
        let (roots, base) = temp_roots("traverse");
        let deep = roots.write.join("sub").join("deep");
        let _ = std::fs::create_dir_all(&deep);
        let escaped = enforce_write_path(&roots, &deep, "..\\..\\..\\evil.png");
        assert!(escaped.is_err());
        let inside = enforce_write_path(&roots, &deep, "tile.png");
        assert!(inside.is_ok());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn link_inside_root_leading_out_is_refused() {
        let (roots, base) = temp_roots("link");
        let outside = base.join("outside");
        let _ = std::fs::create_dir_all(&outside);
        let link = roots.write.join("leak");
        let made = std::os::windows::fs::symlink_dir(&outside, &link);
        if made.is_ok() {
            let via = link.join("evil.png");
            let attempt =
                enforce_write_path(&roots, &roots.write, via.display().to_string().as_str());
            assert!(attempt.is_err());
        } else {
            let direct = enforce_write_path(&roots, &roots.write, "still_inside.png");
            assert!(direct.is_ok());
        }
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn mcp_initialize_and_tools_list_return_13_tools() {
        let mut eng = ShaderEngine::default();
        let db = temp_db("tools");
        let (roots, _base) = temp_roots("tools");
        let init = handle_mcp_request(
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize"}),
            &mut eng,
            &db,
            &roots,
        );
        assert!(init.is_some());

        let list = handle_mcp_request(
            &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            &mut eng,
            &db,
            &roots,
        )
        .unwrap_or_default();
        let tools = list["result"]["tools"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0);
        assert_eq!(tools, 13);
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(_base);
    }

    #[test]
    fn mcp_graph_workflow_and_preset_render_roundtrip() {
        let mut eng = ShaderEngine::default();
        let db = temp_db("flow");
        let (roots, base) = temp_roots("flow");

        let created = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": { "name": "create_graph", "arguments": { "name": "agent weave" } }
            }),
            &mut eng,
            &db,
            &roots,
        )
        .unwrap_or_default();
        assert_eq!(created["result"]["isError"], false);
        let gid = created["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        assert!(gid.contains("graph"));

        let added = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": { "name": "add_node", "arguments": { "kind": "voronoi", "params": { "scale": 6.0 } } }
            }),
            &mut eng,
            &db,
            &roots,
        )
        .unwrap_or_default();
        assert_eq!(added["result"]["isError"], false);

        let stats = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": { "name": "eval_preview_stats", "arguments": { "size": 64 } }
            }),
            &mut eng,
            &db,
            &roots,
        )
        .unwrap_or_default();
        assert_eq!(stats["result"]["isError"], false);

        let denied = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "tools/call",
                "params": { "name": "render_png", "arguments": { "size": 64, "map": "albedo", "filename": "C:\\Windows\\nope.png" } }
            }),
            &mut eng,
            &db,
            &roots,
        )
        .unwrap_or_default();
        assert_eq!(denied["result"]["isError"], true);

        let fname = roots.write.join("weave_normal.png").display().to_string();
        let rendered = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "tools/call",
                "params": { "name": "render_png", "arguments": { "size": 64, "map": "normal", "filename": fname } }
            }),
            &mut eng,
            &db,
            &roots,
        )
        .unwrap_or_default();
        assert_eq!(rendered["result"]["isError"], false);
        let text = rendered["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("weave_normal.png"));

        let setdir = roots.write.join("weave_set").display().to_string();
        let exported = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "tools/call",
                "params": { "name": "export_pbr_set", "arguments": { "size": 64, "dir": setdir } }
            }),
            &mut eng,
            &db,
            &roots,
        )
        .unwrap_or_default();
        assert_eq!(exported["result"]["isError"], false);
        let probe = roots.write.join("weave_normal.png");
        assert!(probe.exists());
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(base);
    }
}
