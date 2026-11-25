//! Domain models for ShaderCraft texture graphs: node kinds, parameter
//! specs, graphs, color ramps, and the four built-in material presets.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Every texture operator the engine can evaluate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    ValueNoise,
    GradientNoise,
    Voronoi,
    Fbm,
    LinearGradient,
    RadialGradient,
    SdfCircle,
    SdfBox,
    Threshold,
    Smoothstep,
    Blend,
    Invert,
    Levels,
}

impl NodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ValueNoise => "value_noise",
            Self::GradientNoise => "gradient_noise",
            Self::Voronoi => "voronoi",
            Self::Fbm => "fbm",
            Self::LinearGradient => "linear_gradient",
            Self::RadialGradient => "radial_gradient",
            Self::SdfCircle => "sdf_circle",
            Self::SdfBox => "sdf_box",
            Self::Threshold => "threshold",
            Self::Smoothstep => "smoothstep",
            Self::Blend => "blend",
            Self::Invert => "invert",
            Self::Levels => "levels",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "value_noise" | "valuenoise" => Some(Self::ValueNoise),
            "gradient_noise" | "perlin" => Some(Self::GradientNoise),
            "voronoi" | "cells" => Some(Self::Voronoi),
            "fbm" | "fractal" => Some(Self::Fbm),
            "linear_gradient" | "linear" => Some(Self::LinearGradient),
            "radial_gradient" | "radial" => Some(Self::RadialGradient),
            "sdf_circle" | "circle" => Some(Self::SdfCircle),
            "sdf_box" | "box" => Some(Self::SdfBox),
            "threshold" => Some(Self::Threshold),
            "smoothstep" => Some(Self::Smoothstep),
            "blend" | "mix" => Some(Self::Blend),
            "invert" | "negate" => Some(Self::Invert),
            "levels" => Some(Self::Levels),
            _ => None,
        }
    }

    /// Number of texture inputs this node reads: 0 for sources, 1 for
    /// filters, 2 for the blend combiner.
    pub fn arity(self) -> usize {
        match self {
            Self::ValueNoise
            | Self::GradientNoise
            | Self::Voronoi
            | Self::Fbm
            | Self::LinearGradient
            | Self::RadialGradient
            | Self::SdfCircle
            | Self::SdfBox => 0,
            Self::Threshold | Self::Smoothstep | Self::Invert | Self::Levels => 1,
            Self::Blend => 2,
        }
    }

    /// Short human description shown in the studio and the help docs.
    pub fn blurb(self) -> &'static str {
        match self {
            Self::ValueNoise => "Seeded lattice noise with smooth interpolation",
            Self::GradientNoise => "Perlin-style gradient noise",
            Self::Voronoi => "Cellular pattern with F1 or edge distance",
            Self::Fbm => "Fractal Brownian motion over several octaves",
            Self::LinearGradient => "Straight color ramp across the tile",
            Self::RadialGradient => "Circular falloff from a center point",
            Self::SdfCircle => "Anti-aliased disc from a signed distance field",
            Self::SdfBox => "Anti-aliased rectangle from a signed distance field",
            Self::Threshold => "Hard or soft cutoff of one input",
            Self::Smoothstep => "Smooth remap of one input between two edges",
            Self::Blend => "Combine two inputs (normal, multiply, overlay)",
            Self::Invert => "Flip one input bright to dark",
            Self::Levels => "Input/output range remap with gamma",
        }
    }
}

/// Parameter spec: key, slider minimum, slider maximum, default value.
pub fn param_specs(kind: NodeKind) -> &'static [(&'static str, f32, f32, f32)] {
    match kind {
        NodeKind::ValueNoise => &[
            ("scale", 1.0, 64.0, 8.0),
            ("seed", 0.0, 100.0, 1.0),
            ("tileable", 0.0, 1.0, 1.0),
        ],
        NodeKind::GradientNoise => &[
            ("scale", 1.0, 64.0, 6.0),
            ("seed", 0.0, 100.0, 2.0),
            ("tileable", 0.0, 1.0, 1.0),
        ],
        NodeKind::Voronoi => &[
            ("scale", 1.0, 32.0, 5.0),
            ("seed", 0.0, 100.0, 3.0),
            ("jitter", 0.0, 1.0, 0.8),
            ("mode", 0.0, 1.0, 0.0),
            ("tileable", 0.0, 1.0, 1.0),
        ],
        NodeKind::Fbm => &[
            ("scale", 1.0, 32.0, 4.0),
            ("seed", 0.0, 100.0, 4.0),
            ("octaves", 1.0, 8.0, 4.0),
            ("lacunarity", 1.0, 4.0, 2.0),
            ("gain", 0.0, 1.0, 0.5),
            ("tileable", 0.0, 1.0, 1.0),
        ],
        NodeKind::LinearGradient => &[("angle", 0.0, 360.0, 0.0), ("repeat", 0.0, 4.0, 0.0)],
        NodeKind::RadialGradient => &[
            ("cx", 0.0, 1.0, 0.5),
            ("cy", 0.0, 1.0, 0.5),
            ("repeat", 0.0, 4.0, 0.0),
        ],
        NodeKind::SdfCircle => &[
            ("cx", 0.0, 1.0, 0.5),
            ("cy", 0.0, 1.0, 0.5),
            ("radius", 0.02, 0.5, 0.25),
            ("soft", 0.0, 0.3, 0.05),
        ],
        NodeKind::SdfBox => &[
            ("cx", 0.0, 1.0, 0.5),
            ("cy", 0.0, 1.0, 0.5),
            ("hx", 0.02, 0.5, 0.25),
            ("hy", 0.02, 0.5, 0.18),
            ("soft", 0.0, 0.3, 0.05),
        ],
        NodeKind::Threshold => &[("level", 0.0, 1.0, 0.5), ("soft", 0.0, 0.5, 0.05)],
        NodeKind::Smoothstep => &[("edge0", 0.0, 1.0, 0.3), ("edge1", 0.0, 1.0, 0.7)],
        NodeKind::Blend => &[("mode", 0.0, 2.0, 0.0), ("opacity", 0.0, 1.0, 1.0)],
        NodeKind::Invert => &[],
        NodeKind::Levels => &[
            ("in_low", 0.0, 1.0, 0.0),
            ("in_high", 0.0, 1.0, 1.0),
            ("out_low", 0.0, 1.0, 0.0),
            ("out_high", 0.0, 1.0, 1.0),
            ("gamma", 0.1, 4.0, 1.0),
        ],
    }
}

/// Graph-level output keys accepted by `set_param` with `node_id = "output"`.
pub const OUTPUT_PARAM_KEYS: &[&str] = &[
    "normal_strength",
    "roughness_base",
    "roughness_scale",
    "metallic_base",
    "metallic_scale",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureNode {
    pub id: String,
    pub kind: NodeKind,
    pub params: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub input: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RampStop {
    pub t: f32,
    pub color: [u8; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureGraph {
    pub id: String,
    pub name: String,
    pub nodes: Vec<TextureNode>,
    pub edges: Vec<TextureEdge>,
    pub output: String,
    pub ramp: Vec<RampStop>,
    pub normal_strength: f32,
    pub roughness_base: f32,
    pub roughness_scale: f32,
    pub metallic_base: f32,
    pub metallic_scale: f32,
}

impl TextureGraph {
    pub fn node(&self, id: &str) -> Option<&TextureNode> {
        self.nodes.iter().find(|n| n.id == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphPreset {
    BrushedMetal,
    MossyStone,
    Lava,
    BlueMarble,
}

impl GraphPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BrushedMetal => "brushed_metal",
            Self::MossyStone => "mossy_stone",
            Self::Lava => "lava",
            Self::BlueMarble => "blue_marble",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "brushed_metal" | "brushedmetal" | "metal" => Some(Self::BrushedMetal),
            "mossy_stone" | "mossystone" | "stone" => Some(Self::MossyStone),
            "lava" => Some(Self::Lava),
            "blue_marble" | "bluemarble" | "marble" => Some(Self::BlueMarble),
            _ => None,
        }
    }

    pub fn list() -> &'static [Self] {
        &[
            Self::BrushedMetal,
            Self::MossyStone,
            Self::Lava,
            Self::BlueMarble,
        ]
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Self::BrushedMetal => "Fine directional grain, high metalness, low roughness",
            Self::MossyStone => "Cellular rock softened by mossy fractal growth",
            Self::Lava => "Dark crust cracked by glowing flow lines",
            Self::BlueMarble => "Bold mineral veins over deep blue stone",
        }
    }
}

fn node(id: &str, kind: NodeKind, params: &[(&str, f32)]) -> TextureNode {
    let mut map = HashMap::new();
    for (key, _, _, default) in param_specs(kind) {
        map.insert((*key).to_string(), *default);
    }
    for (key, value) in params {
        map.insert((*key).to_string(), *value);
    }
    TextureNode {
        id: id.to_string(),
        kind,
        params: map,
    }
}

fn edge(id: &str, from: &str, to: &str, input: u32) -> TextureEdge {
    TextureEdge {
        id: id.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        input,
    }
}

fn stop(t: f32, r: u8, g: u8, b: u8) -> RampStop {
    RampStop {
        t,
        color: [r, g, b],
    }
}

fn base_graph(id: &str, name: &str) -> TextureGraph {
    TextureGraph {
        id: id.to_string(),
        name: name.to_string(),
        nodes: Vec::new(),
        edges: Vec::new(),
        output: String::new(),
        ramp: vec![stop(0.0, 0, 0, 0), stop(1.0, 255, 255, 255)],
        normal_strength: 2.0,
        roughness_base: 0.6,
        roughness_scale: 0.3,
        metallic_base: 0.0,
        metallic_scale: 0.0,
    }
}

/// Build a ready-to-render preset graph. All parameters are fixed numbers,
/// so every preset renders byte-identical output on every machine.
pub fn build_preset(preset: GraphPreset) -> TextureGraph {
    match preset {
        GraphPreset::BrushedMetal => {
            let mut g = base_graph("preset_brushed_metal", "Brushed Metal");
            g.nodes = vec![
                node(
                    "grain",
                    NodeKind::Fbm,
                    &[
                        ("scale", 28.0),
                        ("seed", 11.0),
                        ("octaves", 4.0),
                        ("lacunarity", 2.0),
                        ("gain", 0.5),
                        ("tileable", 1.0),
                    ],
                ),
                node(
                    "finish",
                    NodeKind::Levels,
                    &[
                        ("in_low", 0.25),
                        ("in_high", 0.75),
                        ("out_low", 0.0),
                        ("out_high", 1.0),
                        ("gamma", 1.0),
                    ],
                ),
            ];
            g.edges = vec![edge("e1", "grain", "finish", 0)];
            g.output = "finish".to_string();
            g.ramp = vec![
                stop(0.0, 58, 63, 68),
                stop(0.4, 120, 126, 133),
                stop(0.7, 178, 184, 191),
                stop(1.0, 226, 231, 236),
            ];
            g.normal_strength = 1.5;
            g.roughness_base = 0.38;
            g.roughness_scale = 0.25;
            g.metallic_base = 0.85;
            g.metallic_scale = 0.0;
            g
        }
        GraphPreset::MossyStone => {
            let mut g = base_graph("preset_mossy_stone", "Mossy Stone");
            g.nodes = vec![
                node(
                    "cells",
                    NodeKind::Voronoi,
                    &[
                        ("scale", 6.0),
                        ("seed", 21.0),
                        ("jitter", 0.8),
                        ("mode", 0.0),
                        ("tileable", 1.0),
                    ],
                ),
                node(
                    "moss",
                    NodeKind::Fbm,
                    &[
                        ("scale", 3.0),
                        ("seed", 22.0),
                        ("octaves", 4.0),
                        ("lacunarity", 2.0),
                        ("gain", 0.5),
                        ("tileable", 1.0),
                    ],
                ),
                node(
                    "surface",
                    NodeKind::Blend,
                    &[("mode", 2.0), ("opacity", 0.65)],
                ),
            ];
            g.edges = vec![
                edge("e1", "cells", "surface", 0),
                edge("e2", "moss", "surface", 1),
            ];
            g.output = "surface".to_string();
            g.ramp = vec![
                stop(0.0, 58, 60, 52),
                stop(0.45, 110, 112, 96),
                stop(0.7, 96, 110, 70),
                stop(1.0, 150, 160, 120),
            ];
            g.normal_strength = 2.5;
            g.roughness_base = 0.8;
            g.roughness_scale = 0.15;
            g.metallic_base = 0.0;
            g.metallic_scale = 0.0;
            g
        }
        GraphPreset::Lava => {
            let mut g = base_graph("preset_lava", "Lava");
            g.nodes = vec![
                node(
                    "magma",
                    NodeKind::Fbm,
                    &[
                        ("scale", 4.0),
                        ("seed", 31.0),
                        ("octaves", 5.0),
                        ("lacunarity", 2.0),
                        ("gain", 0.5),
                        ("tileable", 1.0),
                    ],
                ),
                node(
                    "cracks",
                    NodeKind::Voronoi,
                    &[
                        ("scale", 7.0),
                        ("seed", 32.0),
                        ("jitter", 0.9),
                        ("mode", 1.0),
                        ("tileable", 1.0),
                    ],
                ),
                node(
                    "crust",
                    NodeKind::Threshold,
                    &[("level", 0.45), ("soft", 0.15)],
                ),
                node(
                    "flow",
                    NodeKind::Smoothstep,
                    &[("edge0", 0.0), ("edge1", 0.35)],
                ),
                node(
                    "surface",
                    NodeKind::Blend,
                    &[("mode", 0.0), ("opacity", 0.55)],
                ),
            ];
            g.edges = vec![
                edge("e1", "magma", "crust", 0),
                edge("e2", "cracks", "flow", 0),
                edge("e3", "crust", "surface", 0),
                edge("e4", "flow", "surface", 1),
            ];
            g.output = "surface".to_string();
            g.ramp = vec![
                stop(0.0, 12, 8, 8),
                stop(0.35, 120, 20, 8),
                stop(0.65, 230, 90, 15),
                stop(0.85, 255, 180, 60),
                stop(1.0, 255, 240, 200),
            ];
            g.normal_strength = 3.0;
            g.roughness_base = 0.7;
            g.roughness_scale = 0.2;
            g.metallic_base = 0.1;
            g.metallic_scale = 0.0;
            g
        }
        GraphPreset::BlueMarble => {
            let mut g = base_graph("preset_blue_marble", "Blue Marble");
            g.nodes = vec![
                node(
                    "veins",
                    NodeKind::Fbm,
                    &[
                        ("scale", 3.0),
                        ("seed", 41.0),
                        ("octaves", 6.0),
                        ("lacunarity", 2.0),
                        ("gain", 0.55),
                        ("tileable", 1.0),
                    ],
                ),
                node(
                    "band",
                    NodeKind::Smoothstep,
                    &[("edge0", 0.42), ("edge1", 0.58)],
                ),
                node(
                    "marble",
                    NodeKind::Blend,
                    &[("mode", 2.0), ("opacity", 0.5)],
                ),
            ];
            g.edges = vec![
                edge("e1", "veins", "band", 0),
                edge("e2", "veins", "marble", 0),
                edge("e3", "band", "marble", 1),
            ];
            g.output = "marble".to_string();
            g.ramp = vec![
                stop(0.0, 10, 20, 60),
                stop(0.4, 30, 60, 130),
                stop(0.65, 120, 150, 200),
                stop(0.85, 210, 225, 240),
                stop(1.0, 245, 248, 250),
            ];
            g.normal_strength = 1.0;
            g.roughness_base = 0.25;
            g.roughness_scale = 0.2;
            g.metallic_base = 0.05;
            g.metallic_scale = 0.0;
            g
        }
    }
}
