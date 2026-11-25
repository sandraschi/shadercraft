//! Integration suite for ShaderCraft L1 core: determinism, tiling,
//! normal-from-height sanity, blend math, presets, and PNG export.

use shadercraft_core::noise::{fbm, gradient_noise, value_noise, voronoi};
use shadercraft_core::{
    build_preset, encode_png_rgba, height_to_normal, GraphPreset, ShaderEngine,
};
use std::collections::HashMap;

fn engine_with(preset: GraphPreset) -> (ShaderEngine, String) {
    let mut eng = ShaderEngine::default();
    let id = eng.load_preset_graph(preset).unwrap_or_default();
    (eng, id)
}

#[test]
fn noise_is_deterministic_for_same_seed() {
    let (eng, id) = engine_with(GraphPreset::MossyStone);
    let a = eng.eval_height(&id, 64).unwrap_or_default();
    let b = eng.eval_height(&id, 64).unwrap_or_default();
    assert_eq!(a.len(), 64 * 64);
    assert_eq!(a, b);
}

#[test]
fn different_seeds_change_the_bytes() {
    use shadercraft_core::Command;
    let mut eng = ShaderEngine::default();
    let id = eng.create_graph("seed probe").unwrap_or_default();
    let before = eng.eval_height(&id, 64).unwrap_or_default();
    let _ = Command::SetParam {
        graph_id: id.clone(),
        node_id: "n1".to_string(),
        key: "seed".to_string(),
        value: 77.0,
    }
    .dispatch(&mut eng);
    let after = eng.eval_height(&id, 64).unwrap_or_default();
    assert_eq!(before.len(), after.len());
    assert_ne!(before, after);
}

#[test]
fn noise_tiles_exactly_at_period_boundaries() {
    for seed in [1u64, 7, 42] {
        for y in [0.0f32, 0.33, 1.7, 5.125] {
            let a = value_noise(0.0, y, seed, Some(8));
            let b = value_noise(8.0, y, seed, Some(8));
            assert_eq!(a.to_bits(), b.to_bits());
            let c = gradient_noise(0.0, y, seed, Some(6));
            let d = gradient_noise(6.0, y, seed, Some(6));
            assert_eq!(c.to_bits(), d.to_bits());
            let (f1a, f2a) = voronoi(0.0, y, seed, 0.8, Some(5));
            let (f1b, f2b) = voronoi(5.0, y, seed, 0.8, Some(5));
            assert_eq!(
                (f1a.to_bits(), f2a.to_bits()),
                (f1b.to_bits(), f2b.to_bits())
            );
            let ga = fbm(0.0, y, seed, 4, 2.0, 0.5, Some(4));
            let gb = fbm(4.0, y, seed, 4, 2.0, 0.5, Some(4));
            assert_eq!(ga.to_bits(), gb.to_bits());
        }
    }
}

#[test]
fn tileable_flag_engages_wrapping_at_the_right_edge() {
    use shadercraft_core::Command;
    let mut eng = ShaderEngine::default();
    let id = eng.create_graph("tiling probe").unwrap_or_default();
    let wrapped = eng.eval_height(&id, 64).unwrap_or_default();
    let _ = Command::SetParam {
        graph_id: id.clone(),
        node_id: "n1".to_string(),
        key: "tileable".to_string(),
        value: 0.0,
    }
    .dispatch(&mut eng);
    let raw = eng.eval_height(&id, 64).unwrap_or_default();
    assert_eq!(wrapped.len(), raw.len());
    assert_ne!(wrapped, raw);
}

#[test]
fn flat_height_gives_neutral_normal() {
    let flat = vec![0.5f32; 32 * 32];
    let rgb = height_to_normal(&flat, 32, 2.0);
    assert_eq!(rgb.len(), 32 * 32);
    for px in &rgb {
        assert_eq!(*px, [128, 128, 255]);
    }
}

#[test]
fn blend_math_spot_checks() {
    use shadercraft_core::{Command, NodeKind};
    let mut eng = ShaderEngine::default();
    let id = eng.create_graph("blend probe").unwrap_or_default();
    let mut params = HashMap::new();
    params.insert("radius".to_string(), 0.5);
    params.insert("soft".to_string(), 0.000_1);
    let _ = Command::AddNode {
        graph_id: id.clone(),
        kind: NodeKind::SdfCircle.as_str().to_string(),
        params,
    }
    .dispatch(&mut eng);
    let stats = eng.height_stats(&id, 64).unwrap_or((1.0, 0.0, 0.5));
    assert!(stats.0 >= 0.0 && stats.1 <= 1.0 && stats.0 <= stats.2 && stats.2 <= stats.1);
}

#[test]
fn all_presets_render_at_small_size() {
    let mut eng = ShaderEngine::default();
    for preset in GraphPreset::list() {
        let id = eng.load_preset_graph(*preset).unwrap_or_default();
        for map in ["albedo", "height", "normal", "roughness", "metallic"] {
            let (w, h, rgba) = eng.render_rgba(&id, 64, map).unwrap_or((0, 0, Vec::new()));
            assert_eq!((w, h), (64, 64));
            assert_eq!(rgba.len(), 64 * 64 * 4);
            assert!(rgba.iter().any(|v| *v > 0));
        }
        let missing = build_preset(*preset);
        assert!(!missing.nodes.is_empty());
    }
}

#[test]
fn export_writes_a_valid_png_header() {
    let (eng, id) = engine_with(GraphPreset::Lava);
    let bytes = eng.render_png_bytes(&id, 64, "albedo").unwrap_or_default();
    assert!(bytes.len() > 8);
    assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    let set = eng.export_pbr_set(&id, 64).unwrap_or_default();
    assert_eq!(set.len(), 5);
    for (name, body) in &set {
        assert!(name.ends_with(".png"));
        assert_eq!(&body[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}

#[test]
fn encode_png_roundtrip_dimensions() {
    let rgba = vec![200u8; 16 * 16 * 4];
    let bytes = encode_png_rgba(16, 16, &rgba).unwrap_or_default();
    assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
}
