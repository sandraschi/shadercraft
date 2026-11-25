//! Bundled SQLite persistence for ShaderCraft (`rusqlite` 0.41).

use crate::error::ShaderError;
use crate::ShaderEngine;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

pub fn default_db_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("shadercraft").join("shadercraft.db")
}

pub fn save_to_db(engine: &ShaderEngine, path: &Path) -> Result<(), ShaderError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ShaderError::Storage(format!("create_dir_all failed: {e}")))?;
        }
    }
    let mut conn =
        Connection::open(path).map_err(|e| ShaderError::Storage(format!("open db failed: {e}")))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS graphs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            graph_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS engine_meta (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            active_graph TEXT NOT NULL
        );",
    )
    .map_err(|e| ShaderError::Storage(format!("schema init failed: {e}")))?;

    let tx = conn
        .transaction()
        .map_err(|e| ShaderError::Storage(format!("tx start failed: {e}")))?;

    tx.execute("DELETE FROM graphs", [])
        .map_err(|e| ShaderError::Storage(format!("clear graphs: {e}")))?;
    tx.execute("DELETE FROM engine_meta", [])
        .map_err(|e| ShaderError::Storage(format!("clear meta: {e}")))?;

    for g in engine.graphs.values() {
        let body =
            serde_json::to_string(g).map_err(|e| ShaderError::Serialization(e.to_string()))?;
        tx.execute(
            "INSERT INTO graphs (id, name, graph_json) VALUES (?1, ?2, ?3)",
            params![g.id, g.name, body],
        )
        .map_err(|e| ShaderError::Storage(format!("insert graph {}: {e}", g.id)))?;
    }

    tx.execute(
        "INSERT INTO engine_meta (id, active_graph) VALUES (1, ?1)",
        params![engine.active_graph],
    )
    .map_err(|e| ShaderError::Storage(format!("insert meta: {e}")))?;

    tx.commit()
        .map_err(|e| ShaderError::Storage(format!("tx commit: {e}")))?;
    Ok(())
}

pub fn load_from_db(path: &Path) -> Result<Option<ShaderEngine>, ShaderError> {
    if !path.exists() {
        return Ok(None);
    }
    let conn =
        Connection::open(path).map_err(|e| ShaderError::Storage(format!("open db failed: {e}")))?;

    let has_table: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='graphs')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| ShaderError::Storage(format!("table check failed: {e}")))?;
    if !has_table {
        return Ok(None);
    }

    let mut stmt = conn
        .prepare("SELECT id, name, graph_json FROM graphs ORDER BY rowid ASC")
        .map_err(|e| ShaderError::Storage(format!("prepare graphs: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let body: String = row.get(2)?;
            Ok((id, body))
        })
        .map_err(|e| ShaderError::Storage(format!("query graphs: {e}")))?;

    let mut engine = ShaderEngine {
        graphs: std::collections::HashMap::new(),
        active_graph: String::new(),
    };
    for row in rows {
        let (id, body) = row.map_err(|e| ShaderError::Storage(format!("graph row: {e}")))?;
        let graph: crate::TextureGraph = serde_json::from_str(&body)
            .map_err(|e| ShaderError::Serialization(format!("graph {id}: {e}")))?;
        engine.graphs.insert(id, graph);
    }
    if engine.graphs.is_empty() {
        return Ok(None);
    }

    let active: Option<String> = conn
        .query_row(
            "SELECT active_graph FROM engine_meta WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .ok();
    if let Some(want) = active {
        if engine.graphs.contains_key(&want) {
            engine.active_graph = want;
        }
    }
    if engine.active_graph.is_empty() || !engine.graphs.contains_key(&engine.active_graph) {
        let mut ids: Vec<String> = engine.graphs.keys().cloned().collect();
        ids.sort();
        if let Some(first) = ids.first() {
            engine.active_graph = first.clone();
        }
    }
    Ok(Some(engine))
}
