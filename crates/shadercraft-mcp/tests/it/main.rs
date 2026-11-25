//! Binary-level smoke suite for `shadercraft-mcp`: CLI status output and
//! a live MCP `2026-07-28` initialize roundtrip over stdio.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn temp_path(prefix: &str, ext: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("{prefix}_{}{ext}", std::process::id()));
    p
}

#[test]
fn cli_status_reports_shadercraft_ready() {
    let exe = env!("CARGO_BIN_EXE_shadercraft-mcp");
    let db = temp_path("shader_mcp_it", ".db");
    let out = Command::new(exe)
        .args(["--db", &db.display().to_string(), "status"])
        .output();
    assert!(out.is_ok());
    if let Ok(o) = out {
        assert!(o.status.success());
        let text = String::from_utf8_lossy(&o.stdout).into_owned();
        assert!(text.contains("shadercraft"));
        assert!(text.contains("ready"));
    }
    let _ = std::fs::remove_file(&db);
}

#[test]
fn mcp_server_answers_initialize_over_stdio() {
    let exe = env!("CARGO_BIN_EXE_shadercraft-mcp");
    let read_root = std::env::temp_dir();
    let write_root = temp_path("shader_mcp_it_roots", "");
    let _ = std::fs::create_dir_all(&write_root);
    let db = temp_path("shader_mcp_it_mcp", ".db");
    let child = Command::new(exe)
        .args([
            "--db",
            &db.display().to_string(),
            "--mcp",
            "--automation-read-root",
            &read_root.display().to_string(),
            "--automation-write-root",
            &write_root.display().to_string(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    assert!(child.is_ok());
    if let Ok(mut kid) = child {
        let req = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n";
        if let Some(stdin) = kid.stdin.as_mut() {
            let _ = stdin.write_all(req.as_bytes());
            let _ = stdin.flush();
        }
        if let Some(stdout) = kid.stdout.take() {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let got = reader.read_line(&mut line);
            assert!(got.is_ok());
            assert!(line.contains("2026-07-28"));
            assert!(line.contains("shadercraft-mcp"));
        }
        let _ = kid.kill();
        let _ = kid.wait();
    }
    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_dir_all(&write_root);
}

#[test]
fn mcp_server_refuses_to_start_without_roots() {
    let exe = env!("CARGO_BIN_EXE_shadercraft-mcp");
    let out = Command::new(exe).args(["--mcp"]).output();
    assert!(out.is_ok());
    if let Ok(o) = out {
        assert!(!o.status.success());
        let text = String::from_utf8_lossy(&o.stderr).into_owned();
        assert!(text.contains("automation"));
    }
}
