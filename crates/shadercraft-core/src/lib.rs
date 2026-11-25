//! ShaderCraft L1 Core Engine & Universal Command Bus.
//!
//! Deterministic CPU node-graph texture synthesis with height-to-normal
//! conversion and PBR map derivation (albedo, height, normal, roughness,
//! metallic). Same seeds always produce the same bytes: presets render
//! identically in the studio, the CLI, and agent automation.
#![forbid(unsafe_code)]

pub mod db;
pub mod error;
pub mod model;
pub mod noise;

pub use db::{default_db_path, load_from_db, save_to_db};
pub use error::ShaderError;
pub use model::{
    build_preset, param_specs, GraphPreset, NodeKind, RampStop, TextureEdge, TextureGraph,
    TextureNode, OUTPUT_PARAM_KEYS,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Smallest and largest tile edge length accepted by render commands.
pub const MIN_RENDER_SIZE: u32 = 64;
pub const MAX_RENDER_SIZE: u32 = 1024;

/// PBR map names accepted by render and export commands.
pub const MAP_NAMES: &[&str] = &["albedo", "height", "normal", "roughness", "metallic"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderEngine {
    pub graphs: HashMap<String, TextureGraph>,
    pub active_graph: String,
}

impl ShaderEngine {
    fn seeded() -> Self {
        let preset = build_preset(GraphPreset::BrushedMetal);
        let id = preset.id.clone();
        let mut graphs = HashMap::new();
        graphs.insert(id.clone(), preset);
        Self {
            graphs,
            active_graph: id,
        }
    }

    pub fn open_or_seed(path: &std::path::Path) -> Result<Self, ShaderError> {
        if let Some(loaded) = load_from_db(path)? {
            return Ok(loaded);
        }
        let seeded = Self::seeded();
        save_to_db(&seeded, path)?;
        Ok(seeded)
    }

    pub fn save_db(&self, path: &std::path::Path) -> Result<(), ShaderError> {
        save_to_db(self, path)
    }

    pub fn graph(&self, id: &str) -> Result<&TextureGraph, ShaderError> {
        self.graphs
            .get(id)
            .ok_or_else(|| ShaderError::NotFound(format!("graph '{id}'")))
    }

    fn graph_mut(&mut self, id: &str) -> Result<&mut TextureGraph, ShaderError> {
        self.graphs
            .get_mut(id)
            .ok_or_else(|| ShaderError::NotFound(format!("graph '{id}'")))
    }

    fn active(&self) -> Result<&TextureGraph, ShaderError> {
        self.graph(&self.active_graph)
    }

    pub fn create_graph(&mut self, name: &str) -> Result<String, ShaderError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(ShaderError::Validation(
                "Graph name must not be empty".to_string(),
            ));
        }
        let mut n = self.graphs.len() + 1;
        let mut id = format!("graph{n}");
        while self.graphs.contains_key(&id) {
            n += 1;
            id = format!("graph{n}");
        }
        let mut graph = TextureGraph {
            id: id.clone(),
            name: trimmed.to_string(),
            nodes: vec![
                TextureNode {
                    id: "n1".to_string(),
                    kind: NodeKind::ValueNoise,
                    params: default_params(NodeKind::ValueNoise),
                },
                TextureNode {
                    id: "n2".to_string(),
                    kind: NodeKind::Levels,
                    params: default_params(NodeKind::Levels),
                },
            ],
            edges: vec![TextureEdge {
                id: "e1".to_string(),
                from: "n1".to_string(),
                to: "n2".to_string(),
                input: 0,
            }],
            output: "n2".to_string(),
            ramp: vec![
                RampStop {
                    t: 0.0,
                    color: [0, 0, 0],
                },
                RampStop {
                    t: 1.0,
                    color: [255, 255, 255],
                },
            ],
            normal_strength: 2.0,
            roughness_base: 0.6,
            roughness_scale: 0.3,
            metallic_base: 0.0,
            metallic_scale: 0.0,
        };
        if let Some(seed_node) = graph.nodes.first_mut() {
            seed_node.params.insert("scale".to_string(), 8.0);
        }
        self.graphs.insert(id.clone(), graph);
        self.active_graph = id.clone();
        Ok(id)
    }

    pub fn load_preset_graph(&mut self, preset: GraphPreset) -> Result<String, ShaderError> {
        let mut graph = build_preset(preset);
        let base = graph.id.clone();
        let mut n = 1;
        while self.graphs.contains_key(&graph.id) {
            n += 1;
            graph.id = format!("{base}_{n}");
        }
        let id = graph.id.clone();
        self.graphs.insert(id.clone(), graph);
        self.active_graph = id.clone();
        Ok(id)
    }

    pub fn add_node(
        &mut self,
        graph_id: &str,
        kind_raw: &str,
        params: &HashMap<String, f32>,
    ) -> Result<TextureNode, ShaderError> {
        let kind = NodeKind::parse(kind_raw).ok_or_else(|| {
            ShaderError::Validation(format!(
                "Unknown node kind '{kind_raw}'. Expected one of: value_noise, gradient_noise, voronoi, fbm, linear_gradient, radial_gradient, sdf_circle, sdf_box, threshold, smoothstep, blend, invert, levels."
            ))
        })?;
        let specs = param_specs(kind);
        for (key, value) in params {
            let spec = specs.iter().find(|(k, _, _, _)| *k == key.as_str());
            let Some((_, min, max, _)) = spec else {
                return Err(ShaderError::Validation(format!(
                    "Unknown parameter '{key}' for node kind '{}'",
                    kind.as_str()
                )));
            };
            if *value < *min || *value > *max {
                return Err(ShaderError::Validation(format!(
                    "Parameter '{key}' must be between {min} and {max}"
                )));
            }
        }
        let graph = self.graph_mut(graph_id)?;
        let mut n = graph.nodes.len() + 1;
        let mut id = format!("n{n}");
        while graph.nodes.iter().any(|x| x.id == id) {
            n += 1;
            id = format!("n{n}");
        }
        let mut full = default_params(kind);
        for (key, value) in params {
            full.insert(key.clone(), *value);
        }
        let created = TextureNode {
            id: id.clone(),
            kind,
            params: full,
        };
        graph.nodes.push(created.clone());
        Ok(created)
    }

    pub fn set_param(
        &mut self,
        graph_id: &str,
        node_id: &str,
        key: &str,
        value: f32,
    ) -> Result<(), ShaderError> {
        let graph = self.graph_mut(graph_id)?;
        if node_id == "output" {
            if !OUTPUT_PARAM_KEYS.contains(&key) {
                return Err(ShaderError::Validation(format!(
                    "Unknown graph output parameter '{key}'. Expected one of: normal_strength, roughness_base, roughness_scale, metallic_base, metallic_scale."
                )));
            }
            let (min, max) = if key == "normal_strength" {
                (0.1, 16.0)
            } else {
                (0.0, 1.0)
            };
            if value < min || value > max {
                return Err(ShaderError::Validation(format!(
                    "Graph output parameter '{key}' must be between {min} and {max}"
                )));
            }
            match key {
                "normal_strength" => graph.normal_strength = value,
                "roughness_base" => graph.roughness_base = value,
                "roughness_scale" => graph.roughness_scale = value,
                "metallic_base" => graph.metallic_base = value,
                "metallic_scale" => graph.metallic_scale = value,
                _ => {}
            }
            return Ok(());
        }
        let pos = graph
            .nodes
            .iter()
            .position(|x| x.id == node_id)
            .ok_or_else(|| ShaderError::NotFound(format!("node '{node_id}'")))?;
        let kind = graph.nodes[pos].kind;
        let spec = param_specs(kind)
            .iter()
            .find(|(k, _, _, _)| *k == key)
            .ok_or_else(|| {
                ShaderError::Validation(format!(
                    "Unknown parameter '{key}' for node kind '{}'",
                    kind.as_str()
                ))
            })?;
        if value < spec.1 || value > spec.2 {
            return Err(ShaderError::Validation(format!(
                "Parameter '{key}' must be between {} and {}",
                spec.1, spec.2
            )));
        }
        graph.nodes[pos].params.insert(key.to_string(), value);
        Ok(())
    }

    pub fn connect_nodes(
        &mut self,
        graph_id: &str,
        from: &str,
        to: &str,
        input: u32,
    ) -> Result<TextureEdge, ShaderError> {
        let graph = self.graph_mut(graph_id)?;
        if graph.node(from).is_none() {
            return Err(ShaderError::NotFound(format!("node '{from}'")));
        }
        let target = graph
            .node(to)
            .ok_or_else(|| ShaderError::NotFound(format!("node '{to}'")))?;
        if input as usize >= target.kind.arity() {
            return Err(ShaderError::Validation(format!(
                "Node kind '{}' takes {} input(s); slot {input} is out of range",
                target.kind.as_str(),
                target.kind.arity()
            )));
        }
        graph.edges.retain(|e| !(e.to == to && e.input == input));
        let mut n = graph.edges.len() + 1;
        let mut id = format!("e{n}");
        while graph.edges.iter().any(|x| x.id == id) {
            n += 1;
            id = format!("e{n}");
        }
        let created = TextureEdge {
            id,
            from: from.to_string(),
            to: to.to_string(),
            input,
        };
        graph.edges.push(created.clone());
        Ok(created)
    }

    pub fn delete_node(&mut self, graph_id: &str, node_id: &str) -> Result<(), ShaderError> {
        let graph = self.graph_mut(graph_id)?;
        if graph.output == node_id {
            return Err(ShaderError::Validation(format!(
                "Cannot delete node '{node_id}': it is the graph output. Point the graph at another node first."
            )));
        }
        let before = graph.nodes.len();
        graph.nodes.retain(|x| x.id != node_id);
        if graph.nodes.len() == before {
            return Err(ShaderError::NotFound(format!("node '{node_id}'")));
        }
        graph.edges.retain(|e| e.from != node_id && e.to != node_id);
        Ok(())
    }

    /// Evaluate the graph output to a heightfield in `[0, 1]`, row-major.
    pub fn eval_height(&self, graph_id: &str, size: u32) -> Result<Vec<f32>, ShaderError> {
        let size = check_size(size)?;
        let graph = self.graph(graph_id)?;
        if graph.output.trim().is_empty() {
            return Err(ShaderError::Validation(format!(
                "Graph '{graph_id}' has no output node"
            )));
        }
        let mut memo = HashMap::new();
        let mut visiting = Vec::new();
        eval_node(graph, &graph.output, size, &mut memo, &mut visiting)
    }

    pub fn height_stats(&self, graph_id: &str, size: u32) -> Result<(f32, f32, f32), ShaderError> {
        let field = self.eval_height(graph_id, size)?;
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        let mut sum = 0.0;
        for v in &field {
            if *v < min {
                min = *v;
            }
            if *v > max {
                max = *v;
            }
            sum += *v;
        }
        let count = field.len().max(1) as f32;
        Ok((min, max, sum / count))
    }

    /// Render one PBR map to 8-bit RGBA bytes, row-major.
    pub fn render_rgba(
        &self,
        graph_id: &str,
        size: u32,
        map: &str,
    ) -> Result<(u32, u32, Vec<u8>), ShaderError> {
        let size_usize = check_size(size)?;
        let graph = self.graph(graph_id)?;
        let key = map.trim().to_ascii_lowercase();
        if !MAP_NAMES.contains(&key.as_str()) {
            return Err(ShaderError::Validation(format!(
                "Unknown map '{map}'. Expected one of: albedo, height, normal, roughness, metallic."
            )));
        }
        let height = self.eval_height(graph_id, size)?;
        let edge = size_usize as u32;
        let mut rgba = vec![0u8; size_usize * size_usize * 4];
        if key == "normal" {
            let rgb = height_to_normal(&height, size_usize, graph.normal_strength);
            for (i, px) in rgb.iter().enumerate() {
                let o = i * 4;
                rgba[o] = px[0];
                rgba[o + 1] = px[1];
                rgba[o + 2] = px[2];
                rgba[o + 3] = 255;
            }
        } else {
            for (i, h) in height.iter().enumerate() {
                let o = i * 4;
                let (r, g, b) = match key.as_str() {
                    "albedo" => ramp_color(&graph.ramp, *h),
                    "height" => {
                        let v = (*h * 255.0).round().clamp(0.0, 255.0) as u8;
                        (v, v, v)
                    }
                    "roughness" => {
                        let v = (graph.roughness_base + (1.0 - h) * graph.roughness_scale)
                            .clamp(0.0, 1.0);
                        let b = (v * 255.0).round().clamp(0.0, 255.0) as u8;
                        (b, b, b)
                    }
                    _ => {
                        let v = (graph.metallic_base + h * graph.metallic_scale).clamp(0.0, 1.0);
                        let b = (v * 255.0).round().clamp(0.0, 255.0) as u8;
                        (b, b, b)
                    }
                };
                rgba[o] = r;
                rgba[o + 1] = g;
                rgba[o + 2] = b;
                rgba[o + 3] = 255;
            }
        }
        Ok((edge, edge, rgba))
    }

    pub fn render_png_bytes(
        &self,
        graph_id: &str,
        size: u32,
        map: &str,
    ) -> Result<Vec<u8>, ShaderError> {
        let (w, h, rgba) = self.render_rgba(graph_id, size, map)?;
        encode_png_rgba(w, h, &rgba)
    }

    /// Render all five PBR maps. Returns `(file_name, png_bytes)` pairs; the
    /// caller (CLI, MCP server, studio) performs the file writes so automation
    /// roots can be enforced at the boundary.
    pub fn export_pbr_set(
        &self,
        graph_id: &str,
        size: u32,
    ) -> Result<Vec<(String, Vec<u8>)>, ShaderError> {
        let graph = self.graph(graph_id)?;
        let stem = pbr_file_stem(&graph.name);
        let mut out = Vec::new();
        for map in MAP_NAMES {
            let bytes = self.render_png_bytes(graph_id, size, map)?;
            out.push((format!("{stem}_{map}.png"), bytes));
        }
        Ok(out)
    }
}

impl Default for ShaderEngine {
    fn default() -> Self {
        Self::seeded()
    }
}

fn default_params(kind: NodeKind) -> HashMap<String, f32> {
    let mut map = HashMap::new();
    for (key, _, _, default) in param_specs(kind) {
        map.insert((*key).to_string(), *default);
    }
    map
}

pub fn check_size(size: u32) -> Result<usize, ShaderError> {
    if !(MIN_RENDER_SIZE..=MAX_RENDER_SIZE).contains(&size) {
        return Err(ShaderError::Validation(format!(
            "Render size must be between {MIN_RENDER_SIZE} and {MAX_RENDER_SIZE} pixels"
        )));
    }
    Ok(size as usize)
}

fn tile_period(params: &HashMap<String, f32>, scale: f32) -> Option<u32> {
    let tileable = params.get("tileable").copied().unwrap_or(1.0) >= 0.5;
    if tileable {
        Some(scale.round().clamp(1.0, 256.0) as u32)
    } else {
        None
    }
}

fn param(params: &HashMap<String, f32>, key: &str, default: f32) -> f32 {
    params.get(key).copied().unwrap_or(default)
}

fn eval_node(
    graph: &TextureGraph,
    id: &str,
    size: usize,
    memo: &mut HashMap<String, Vec<f32>>,
    visiting: &mut Vec<String>,
) -> Result<Vec<f32>, ShaderError> {
    if let Some(cached) = memo.get(id) {
        return Ok(cached.clone());
    }
    if visiting.iter().any(|v| v.as_str() == id) {
        return Err(ShaderError::CycleDetected(format!(
            "node '{id}' feeds back into itself"
        )));
    }
    let kind = graph
        .node(id)
        .map(|n| n.kind)
        .ok_or_else(|| ShaderError::NotFound(format!("node '{id}'")))?;
    visiting.push(id.to_string());
    let result = eval_node_inner(graph, id, kind, size, memo, visiting);
    let _ = visiting.pop();
    let field = result?;
    memo.insert(id.to_string(), field.clone());
    Ok(field)
}

fn eval_node_inner(
    graph: &TextureGraph,
    id: &str,
    kind: NodeKind,
    size: usize,
    memo: &mut HashMap<String, Vec<f32>>,
    visiting: &mut Vec<String>,
) -> Result<Vec<f32>, ShaderError> {
    let mut inputs: HashMap<u32, Vec<f32>> = HashMap::new();
    for e in graph.edges.iter().filter(|e| e.to == id) {
        if e.input as usize >= kind.arity() {
            return Err(ShaderError::Validation(format!(
                "Edge '{}' targets input slot {} of node kind '{}', which takes {} input(s)",
                e.id,
                e.input,
                kind.as_str(),
                kind.arity()
            )));
        }
        let field = eval_node(graph, &e.from, size, memo, visiting)?;
        inputs.insert(e.input, field);
    }
    for slot in 0..kind.arity() {
        if !inputs.contains_key(&(slot as u32)) {
            return Err(ShaderError::Validation(format!(
                "Node '{id}' of kind '{}' is missing required input {slot}",
                kind.as_str()
            )));
        }
    }
    let n = graph
        .node(id)
        .ok_or_else(|| ShaderError::NotFound(format!("node '{id}'")))?;
    compute_node(n, &inputs, size)
}

fn input_field<'a>(
    inputs: &'a HashMap<u32, Vec<f32>>,
    slot: u32,
    id: &str,
) -> Result<&'a Vec<f32>, ShaderError> {
    inputs.get(&slot).ok_or_else(|| {
        ShaderError::Validation(format!("Node '{id}' is missing required input {slot}"))
    })
}

fn compute_node(
    n: &TextureNode,
    inputs: &HashMap<u32, Vec<f32>>,
    size: usize,
) -> Result<Vec<f32>, ShaderError> {
    let size_f = size as f32;
    let mut out = vec![0.0f32; size * size];
    match n.kind {
        NodeKind::ValueNoise => {
            let scale = param(&n.params, "scale", 8.0).clamp(0.25, 256.0);
            let seed = param(&n.params, "seed", 1.0).round().max(0.0) as u64;
            let period = tile_period(&n.params, scale);
            for (i, px) in out.iter_mut().enumerate() {
                let x = i % size;
                let y = i / size;
                let u = (x as f32 + 0.5) / size_f;
                let v = (y as f32 + 0.5) / size_f;
                *px = noise::value_noise(u * scale, v * scale, seed, period);
            }
        }
        NodeKind::GradientNoise => {
            let scale = param(&n.params, "scale", 6.0).clamp(0.25, 256.0);
            let seed = param(&n.params, "seed", 2.0).round().max(0.0) as u64;
            let period = tile_period(&n.params, scale);
            for (i, px) in out.iter_mut().enumerate() {
                let x = i % size;
                let y = i / size;
                let u = (x as f32 + 0.5) / size_f;
                let v = (y as f32 + 0.5) / size_f;
                *px = noise::gradient_noise(u * scale, v * scale, seed, period);
            }
        }
        NodeKind::Voronoi => {
            let scale = param(&n.params, "scale", 5.0).clamp(0.25, 256.0);
            let seed = param(&n.params, "seed", 3.0).round().max(0.0) as u64;
            let jitter = param(&n.params, "jitter", 0.8).clamp(0.0, 1.0);
            let mode = param(&n.params, "mode", 0.0).round() as i32;
            let period = tile_period(&n.params, scale);
            for (i, px) in out.iter_mut().enumerate() {
                let x = i % size;
                let y = i / size;
                let u = (x as f32 + 0.5) / size_f;
                let v = (y as f32 + 0.5) / size_f;
                let (f1, f2) = noise::voronoi(u * scale, v * scale, seed, jitter, period);
                *px = match mode {
                    1 => ((f2 - f1) * 1.5).clamp(0.0, 1.0),
                    _ => f1.clamp(0.0, 1.0),
                };
            }
        }
        NodeKind::Fbm => {
            let scale = param(&n.params, "scale", 4.0).clamp(0.25, 256.0);
            let seed = param(&n.params, "seed", 4.0).round().max(0.0) as u64;
            let octaves = param(&n.params, "octaves", 4.0).round().clamp(1.0, 8.0) as u32;
            let lacunarity = param(&n.params, "lacunarity", 2.0).clamp(1.0, 4.0);
            let gain = param(&n.params, "gain", 0.5).clamp(0.0, 1.0);
            let period = tile_period(&n.params, scale);
            for (i, px) in out.iter_mut().enumerate() {
                let x = i % size;
                let y = i / size;
                let u = (x as f32 + 0.5) / size_f;
                let v = (y as f32 + 0.5) / size_f;
                *px = noise::fbm(
                    u * scale,
                    v * scale,
                    seed,
                    octaves,
                    lacunarity,
                    gain,
                    period,
                );
            }
        }
        NodeKind::LinearGradient => {
            let angle = param(&n.params, "angle", 0.0).to_radians();
            let repeat = param(&n.params, "repeat", 0.0).round().clamp(0.0, 4.0);
            let (ca, sa) = (angle.cos(), angle.sin());
            for (i, px) in out.iter_mut().enumerate() {
                let x = i % size;
                let y = i / size;
                let u = (x as f32 + 0.5) / size_f;
                let v = (y as f32 + 0.5) / size_f;
                let t = u * ca + v * sa;
                *px = if repeat >= 0.5 {
                    (t * repeat).fract().abs()
                } else {
                    t.clamp(0.0, 1.0)
                };
            }
        }
        NodeKind::RadialGradient => {
            let cx = param(&n.params, "cx", 0.5).clamp(0.0, 1.0);
            let cy = param(&n.params, "cy", 0.5).clamp(0.0, 1.0);
            let repeat = param(&n.params, "repeat", 0.0).round().clamp(0.0, 4.0);
            for (i, px) in out.iter_mut().enumerate() {
                let x = i % size;
                let y = i / size;
                let u = (x as f32 + 0.5) / size_f;
                let v = (y as f32 + 0.5) / size_f;
                let d = ((u - cx).powi(2) + (v - cy).powi(2)).sqrt() * 1.5;
                *px = if repeat >= 0.5 {
                    (d * repeat).fract().abs()
                } else {
                    d.clamp(0.0, 1.0)
                };
            }
        }
        NodeKind::SdfCircle => {
            let cx = param(&n.params, "cx", 0.5).clamp(0.0, 1.0);
            let cy = param(&n.params, "cy", 0.5).clamp(0.0, 1.0);
            let radius = param(&n.params, "radius", 0.25).clamp(0.005, 1.0);
            let soft = param(&n.params, "soft", 0.05).clamp(0.0, 1.0).max(0.0001);
            for (i, px) in out.iter_mut().enumerate() {
                let x = i % size;
                let y = i / size;
                let u = (x as f32 + 0.5) / size_f;
                let v = (y as f32 + 0.5) / size_f;
                let d = ((u - cx).powi(2) + (v - cy).powi(2)).sqrt();
                *px = ((radius - d) / soft).clamp(0.0, 1.0);
            }
        }
        NodeKind::SdfBox => {
            let cx = param(&n.params, "cx", 0.5).clamp(0.0, 1.0);
            let cy = param(&n.params, "cy", 0.5).clamp(0.0, 1.0);
            let hx = param(&n.params, "hx", 0.25).clamp(0.005, 1.0);
            let hy = param(&n.params, "hy", 0.18).clamp(0.005, 1.0);
            let soft = param(&n.params, "soft", 0.05).clamp(0.0, 1.0).max(0.0001);
            for (i, px) in out.iter_mut().enumerate() {
                let x = i % size;
                let y = i / size;
                let u = (x as f32 + 0.5) / size_f;
                let v = (y as f32 + 0.5) / size_f;
                let qx = (u - cx).abs() - hx;
                let qy = (v - cy).abs() - hy;
                let ax = qx.max(0.0);
                let ay = qy.max(0.0);
                let outside = (ax * ax + ay * ay).sqrt() + qx.max(qy).min(0.0);
                *px = ((0.0 - outside) / soft).clamp(0.0, 1.0);
            }
        }
        NodeKind::Threshold => {
            let src = input_field(inputs, 0, &n.id)?;
            let level = param(&n.params, "level", 0.5).clamp(0.0, 1.0);
            let soft = param(&n.params, "soft", 0.05).clamp(0.0, 0.5);
            for (i, px) in out.iter_mut().enumerate() {
                let h = src[i];
                *px = if soft < 0.000_001 {
                    if h >= level {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    ((h - (level - soft)) / (2.0 * soft)).clamp(0.0, 1.0)
                };
            }
        }
        NodeKind::Smoothstep => {
            let src = input_field(inputs, 0, &n.id)?;
            let e0 = param(&n.params, "edge0", 0.3).clamp(0.0, 1.0);
            let e1 = param(&n.params, "edge1", 0.7).clamp(0.0, 1.0);
            if e1 <= e0 {
                return Err(ShaderError::Validation(format!(
                    "Node '{}': edge1 must be greater than edge0",
                    n.id
                )));
            }
            for (i, px) in out.iter_mut().enumerate() {
                let t = ((src[i] - e0) / (e1 - e0)).clamp(0.0, 1.0);
                *px = t * t * (3.0 - 2.0 * t);
            }
        }
        NodeKind::Blend => {
            let a = input_field(inputs, 0, &n.id)?;
            let b = input_field(inputs, 1, &n.id)?;
            let mode = param(&n.params, "mode", 0.0).round() as i32;
            let opacity = param(&n.params, "opacity", 1.0).clamp(0.0, 1.0);
            for (i, px) in out.iter_mut().enumerate() {
                let base = a[i];
                let top = b[i];
                let mixed = match mode {
                    1 => base * top,
                    2 => {
                        if base < 0.5 {
                            2.0 * base * top
                        } else {
                            1.0 - 2.0 * (1.0 - base) * (1.0 - top)
                        }
                    }
                    _ => top,
                };
                *px = (base + (mixed - base) * opacity).clamp(0.0, 1.0);
            }
        }
        NodeKind::Invert => {
            let src = input_field(inputs, 0, &n.id)?;
            for (i, px) in out.iter_mut().enumerate() {
                *px = 1.0 - src[i];
            }
        }
        NodeKind::Levels => {
            let src = input_field(inputs, 0, &n.id)?;
            let in_low = param(&n.params, "in_low", 0.0).clamp(0.0, 1.0);
            let in_high = param(&n.params, "in_high", 1.0).clamp(0.0, 1.0);
            let out_low = param(&n.params, "out_low", 0.0).clamp(0.0, 1.0);
            let out_high = param(&n.params, "out_high", 1.0).clamp(0.0, 1.0);
            let gamma = param(&n.params, "gamma", 1.0).clamp(0.05, 8.0);
            if in_high <= in_low {
                return Err(ShaderError::Validation(format!(
                    "Node '{}': in_high must be greater than in_low",
                    n.id
                )));
            }
            for (i, px) in out.iter_mut().enumerate() {
                let t = ((src[i] - in_low) / (in_high - in_low)).clamp(0.0, 1.0);
                *px = (out_low + (out_high - out_low) * t.powf(gamma)).clamp(0.0, 1.0);
            }
        }
    }
    Ok(out)
}

/// Convert a heightfield to an RGB normal map with `flat height = neutral
/// normal (128, 128, 255)`. Central differences, clamped tile edges.
pub fn height_to_normal(height: &[f32], size: usize, strength: f32) -> Vec<[u8; 3]> {
    let s = strength.clamp(0.1, 16.0);
    let at = |x: usize, y: usize| -> f32 {
        let cx = x.min(size - 1);
        let cy = y.min(size - 1);
        height[cy * size + cx]
    };
    let mut rgb = vec![[128u8; 3]; size * size];
    for y in 0..size {
        for x in 0..size {
            let xl = if x == 0 { 0 } else { x - 1 };
            let xr = if x + 1 >= size { size - 1 } else { x + 1 };
            let yt = if y == 0 { 0 } else { y - 1 };
            let yb = if y + 1 >= size { size - 1 } else { y + 1 };
            let dx = (at(xr, y) - at(xl, y)) * s;
            let dy = (at(x, yb) - at(x, yt)) * s;
            let len = (dx * dx + dy * dy + 1.0).sqrt();
            let r = ((-dx / len) * 0.5 + 0.5) * 255.0;
            let g = ((-dy / len) * 0.5 + 0.5) * 255.0;
            let b = ((1.0 / len) * 0.5 + 0.5) * 255.0;
            rgb[y * size + x] = [
                r.round().clamp(0.0, 255.0) as u8,
                g.round().clamp(0.0, 255.0) as u8,
                b.round().clamp(0.0, 255.0) as u8,
            ];
        }
    }
    rgb
}

fn ramp_color(ramp: &[RampStop], h: f32) -> (u8, u8, u8) {
    if ramp.is_empty() {
        let v = (h * 255.0).round().clamp(0.0, 255.0) as u8;
        return (v, v, v);
    }
    if h <= ramp[0].t {
        return (ramp[0].color[0], ramp[0].color[1], ramp[0].color[2]);
    }
    if h >= ramp[ramp.len() - 1].t {
        let c = ramp[ramp.len() - 1].color;
        return (c[0], c[1], c[2]);
    }
    for pair in ramp.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if h >= a.t && h <= b.t {
            let span = (b.t - a.t).max(0.000_001);
            let t = ((h - a.t) / span).clamp(0.0, 1.0);
            let mix = |x: u8, y: u8| {
                (f32::from(x) + (f32::from(y) - f32::from(x)) * t)
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            return (
                mix(a.color[0], b.color[0]),
                mix(a.color[1], b.color[1]),
                mix(a.color[2], b.color[2]),
            );
        }
    }
    let c = ramp[ramp.len() - 1].color;
    (c[0], c[1], c[2])
}

/// Encode 8-bit RGBA pixels as a PNG file body.
pub fn encode_png_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, ShaderError> {
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;
    let mut buf = Vec::new();
    let enc = PngEncoder::new(&mut buf);
    enc.write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|e| ShaderError::Image(e.to_string()))?;
    Ok(buf)
}

/// File stem for PBR exports derived from the graph name.
pub fn pbr_file_stem(name: &str) -> String {
    let mut stem = String::new();
    for ch in name.trim().to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            stem.push(ch);
        } else if ch == ' ' || ch == '-' || ch == '_' {
            stem.push('_');
        }
    }
    while stem.contains("__") {
        stem = stem.replace("__", "_");
    }
    let trimmed = stem.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "texture".to_string()
    } else {
        trimmed
    }
}

/// Every mutation and query the studio panels, the CLI, and the MCP server
/// can perform. Panels must go through this bus (rule 14).
#[derive(Debug, Clone)]
pub enum Command {
    Status,
    Summary,
    CreateGraph {
        name: String,
    },
    ListGraphs,
    ListPresets,
    LoadPreset {
        preset: String,
    },
    AddNode {
        graph_id: String,
        kind: String,
        params: HashMap<String, f32>,
    },
    SetParam {
        graph_id: String,
        node_id: String,
        key: String,
        value: f32,
    },
    ConnectNodes {
        graph_id: String,
        from: String,
        to: String,
        input: u32,
    },
    DeleteNode {
        graph_id: String,
        node_id: String,
    },
    EvalPreviewStats {
        graph_id: String,
        size: u32,
    },
    RenderPng {
        graph_id: String,
        size: u32,
        map: String,
    },
    ExportPbrSet {
        graph_id: String,
        size: u32,
    },
}

impl Command {
    pub fn dispatch(&self, engine: &mut ShaderEngine) -> Result<Value, ShaderError> {
        match self {
            Self::Status => {
                let active = engine.active()?;
                Ok(json!({
                    "app": "shadercraft",
                    "version": APP_VERSION,
                    "status": "ready",
                    "active_graph": engine.active_graph,
                    "graph_count": engine.graphs.len(),
                    "node_count": active.nodes.len(),
                    "edge_count": active.edges.len(),
                }))
            }
            Self::Summary => {
                let active = engine.active()?;
                Ok(json!({
                    "active_graph": engine.active_graph,
                    "graph": active,
                }))
            }
            Self::CreateGraph { name } => {
                let id = engine.create_graph(name)?;
                let graph = engine.graph(&id)?;
                Ok(json!({
                    "created": id,
                    "name": graph.name,
                    "node_count": graph.nodes.len(),
                    "edge_count": graph.edges.len(),
                    "output": graph.output,
                }))
            }
            Self::ListGraphs => {
                let mut graphs: Vec<Value> = engine
                    .graphs
                    .values()
                    .map(|g| {
                        json!({
                            "id": g.id,
                            "name": g.name,
                            "node_count": g.nodes.len(),
                            "edge_count": g.edges.len(),
                            "output": g.output,
                        })
                    })
                    .collect();
                graphs.sort_by(|a, b| {
                    a["id"]
                        .as_str()
                        .unwrap_or_default()
                        .cmp(b["id"].as_str().unwrap_or_default())
                });
                Ok(json!({ "graphs": graphs, "active_graph": engine.active_graph }))
            }
            Self::ListPresets => {
                let presets: Vec<Value> = GraphPreset::list()
                    .iter()
                    .map(|p| json!({ "id": p.as_str(), "blurb": p.blurb() }))
                    .collect();
                Ok(json!({ "presets": presets }))
            }
            Self::LoadPreset { preset } => {
                let parsed = GraphPreset::parse(preset).ok_or_else(|| {
                    ShaderError::Validation(format!(
                        "Unknown preset '{preset}'. Expected brushed_metal, mossy_stone, lava, or blue_marble."
                    ))
                })?;
                let id = engine.load_preset_graph(parsed)?;
                let graph = engine.graph(&id)?;
                Ok(json!({
                    "loaded": parsed.as_str(),
                    "graph_id": id,
                    "node_count": graph.nodes.len(),
                    "edge_count": graph.edges.len(),
                }))
            }
            Self::AddNode {
                graph_id,
                kind,
                params,
            } => {
                let created = engine.add_node(graph_id, kind, params)?;
                Ok(json!({ "added": created }))
            }
            Self::SetParam {
                graph_id,
                node_id,
                key,
                value,
            } => {
                engine.set_param(graph_id, node_id, key, *value)?;
                Ok(json!({ "graph_id": graph_id, "node_id": node_id, "key": key, "value": value }))
            }
            Self::ConnectNodes {
                graph_id,
                from,
                to,
                input,
            } => {
                let created = engine.connect_nodes(graph_id, from, to, *input)?;
                Ok(json!({ "wire": created }))
            }
            Self::DeleteNode { graph_id, node_id } => {
                engine.delete_node(graph_id, node_id)?;
                Ok(json!({ "removed": node_id }))
            }
            Self::EvalPreviewStats { graph_id, size } => {
                let (min, max, mean) = engine.height_stats(graph_id, *size)?;
                Ok(json!({
                    "graph_id": graph_id,
                    "size": size,
                    "min": min,
                    "max": max,
                    "mean": mean,
                }))
            }
            Self::RenderPng {
                graph_id,
                size,
                map,
            } => {
                let (w, h, rgba) = engine.render_rgba(graph_id, *size, map)?;
                let bytes = encode_png_rgba(w, h, &rgba)?;
                let (min, max, mean) = engine.height_stats(graph_id, *size)?;
                Ok(json!({
                    "graph_id": graph_id,
                    "size": size,
                    "map": map,
                    "width": w,
                    "height": h,
                    "png_bytes": bytes.len(),
                    "min": min,
                    "max": max,
                    "mean": mean,
                }))
            }
            Self::ExportPbrSet { graph_id, size } => {
                let files = engine.export_pbr_set(graph_id, *size)?;
                let names: Vec<String> = files.iter().map(|(name, _)| name.clone()).collect();
                let total: usize = files.iter().map(|(_, b)| b.len()).sum();
                Ok(json!({
                    "graph_id": graph_id,
                    "size": size,
                    "files": names,
                    "total_bytes": total,
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_engine_opens_with_brushed_metal_graph() {
        let eng = ShaderEngine::default();
        assert_eq!(eng.active_graph, "preset_brushed_metal");
        assert!(eng.graph("preset_brushed_metal").is_ok());
    }

    #[test]
    fn preset_graphs_evaluate_inside_unit_range() {
        let eng = ShaderEngine::default();
        for preset in GraphPreset::list() {
            let mut owned = eng.clone();
            let id = owned.load_preset_graph(*preset).unwrap_or_default();
            let field = owned.eval_height(&id, 64).unwrap_or_default();
            assert_eq!(field.len(), 64 * 64);
            assert!(field.iter().all(|v| *v >= 0.0 && *v <= 1.0));
        }
    }

    #[test]
    fn render_size_bounds_are_rejected() {
        assert!(check_size(63).is_err());
        assert!(check_size(1025).is_err());
        assert!(check_size(64).is_ok());
        assert!(check_size(1024).is_ok());
    }

    #[test]
    fn unknown_map_name_is_rejected() {
        let eng = ShaderEngine::default();
        let id = eng.active_graph.clone();
        assert!(eng.render_rgba(&id, 64, "depth").is_err());
        assert!(eng.render_rgba(&id, 64, "normal").is_ok());
    }

    #[test]
    fn sqlite_roundtrip_persists_graphs_and_params() {
        let mut path = std::env::temp_dir();
        path.push(format!("shadercraft_test_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let mut eng = ShaderEngine::default();
        let id = eng.create_graph("roundtrip").unwrap_or_default();
        let _ = Command::SetParam {
            graph_id: id.clone(),
            node_id: "output".to_string(),
            key: "roughness_base".to_string(),
            value: 0.2,
        }
        .dispatch(&mut eng);
        assert!(eng.save_db(&path).is_ok());

        let loaded = load_from_db(&path).ok().flatten();
        let _ = std::fs::remove_file(&path);
        assert!(loaded.is_some());
        if let Some(l) = loaded {
            let g = l.graph(&id).ok();
            assert!(g.is_some());
            if let Some(g) = g {
                assert!((g.roughness_base - 0.2).abs() < 0.000_1);
            }
        }
    }
}
