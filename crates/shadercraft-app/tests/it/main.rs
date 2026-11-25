//! Integration suite for ShaderCraft L3: every preset renders through the
//! same engine the studio drives, and the full graph-edit workflow round
//! trips through `Command::dispatch`.

use shadercraft_core::{build_preset, Command, GraphPreset, ShaderEngine};
use std::collections::HashMap;

#[test]
fn studio_workflow_add_tune_connect_delete() {
    let mut eng = ShaderEngine::default();
    let gid = eng.create_graph("studio probe").unwrap_or_default();
    let created = Command::AddNode {
        graph_id: gid.clone(),
        kind: "voronoi".to_string(),
        params: HashMap::new(),
    }
    .dispatch(&mut eng);
    assert!(created.is_ok());
    let node_id = created
        .ok()
        .and_then(|v| {
            v.get("added")
                .and_then(|a| a.get("id"))
                .and_then(|i| i.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    assert!(!node_id.is_empty());

    let tuned = Command::SetParam {
        graph_id: gid.clone(),
        node_id: node_id.clone(),
        key: "scale".to_string(),
        value: 9.0,
    }
    .dispatch(&mut eng);
    assert!(tuned.is_ok());

    let wired = Command::ConnectNodes {
        graph_id: gid.clone(),
        from: node_id.clone(),
        to: "n2".to_string(),
        input: 0,
    }
    .dispatch(&mut eng);
    assert!(wired.is_ok());

    let stats = Command::EvalPreviewStats {
        graph_id: gid.clone(),
        size: 64,
    }
    .dispatch(&mut eng);
    assert!(stats.is_ok());

    let removed = Command::DeleteNode {
        graph_id: gid.clone(),
        node_id,
    }
    .dispatch(&mut eng);
    assert!(removed.is_ok());
}

#[test]
fn every_preset_renders_all_five_maps() {
    let mut eng = ShaderEngine::default();
    for preset in GraphPreset::list() {
        let gid = eng.load_preset_graph(*preset).unwrap_or_default();
        for map in ["albedo", "height", "normal", "roughness", "metallic"] {
            let bytes = eng.render_png_bytes(&gid, 64, map).unwrap_or_default();
            assert!(bytes.len() > 64);
            assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        }
        let built = build_preset(*preset);
        assert!(!built.ramp.is_empty());
    }
}
