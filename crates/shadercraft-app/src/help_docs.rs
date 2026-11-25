//! Six-tab in-app documentation for ShaderCraft.

use crate::app::{HelpTab, ShaderCraftApp};
use eframe::egui;
use shadercraft_core::{param_specs, GraphPreset, NodeKind, APP_VERSION};

const HELP_MD: &str = include_str!("../../../HELP.md");

const NODE_KINDS: &[NodeKind] = &[
    NodeKind::ValueNoise,
    NodeKind::GradientNoise,
    NodeKind::Voronoi,
    NodeKind::Fbm,
    NodeKind::LinearGradient,
    NodeKind::RadialGradient,
    NodeKind::SdfCircle,
    NodeKind::SdfBox,
    NodeKind::Threshold,
    NodeKind::Smoothstep,
    NodeKind::Blend,
    NodeKind::Invert,
    NodeKind::Levels,
];

pub fn render_help_tab_content(app: &ShaderCraftApp, ui: &mut egui::Ui) {
    match app.help_tab {
        HelpTab::GetStarted => {
            ui.label(HELP_MD);
        }
        HelpTab::GraphNodes => {
            ui.heading("Workbench 1: Texture Graph Nodes & Parameters");
            ui.label(
                "Every ShaderCraft texture is a small graph of deterministic CPU nodes. Sources make fields, filters reshape one field, and the blend node combines two.",
            );
            ui.add_space(6.0);
            egui::Grid::new("help_nodes_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Node Kind");
                    ui.strong("Inputs");
                    ui.strong("What It Does");
                    ui.end_row();
                    for kind in NODE_KINDS {
                        ui.label(kind.as_str());
                        ui.label(format!("{}", kind.arity()));
                        ui.label(kind.blurb());
                        ui.end_row();
                    }
                });
            ui.add_space(6.0);
            ui.label(
                "Seeds are plain numbers: the same graph with the same seed renders byte-identical tiles on every machine. Noise nodes wrap their lattice modulo the scale when tileable is on, so tiles repeat without seams.",
            );
        }
        HelpTab::PreviewExport => {
            ui.heading("Workbench 2 and 3: Preview, Statistics, and PBR Export");
            ui.label(
                "The 2D Preview renders the graph output heightfield through a color ramp (albedo), a height-to-normal Sobel pass, or the scalar roughness and metallic derivations.",
            );
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("The five derived maps");
                ui.label("albedo: heightfield through the graph color ramp.");
                ui.label("height: raw heightfield as gray levels.");
                ui.label("normal: Sobel gradient of the heightfield, neutral (128, 128, 255) on flat areas.");
                ui.label("roughness: roughness_base plus (1 - height) times roughness_scale.");
                ui.label("metallic: metallic_base plus height times metallic_scale.");
            });
            ui.add_space(6.0);
            ui.label(
                "Export writes 8-bit RGBA PNG files. Sizes from 64 to 1024 pixels are accepted; 256 is a good working size and 1024 suits final game art.",
            );
        }
        HelpTab::PresetsCli => {
            ui.heading("Workbench 4: Presets, CLI, and Agent Automation");
            for preset in GraphPreset::list() {
                ui.group(|ui| {
                    ui.strong(preset.as_str());
                    ui.label(preset.blurb());
                });
            }
            ui.add_space(6.0);
            ui.label(
                "Everything the studio does is also available through the CLI and the MCP 2026-07-28 stdio server, which share the same Command dispatch. Agent file writes stay inside the automation write root.",
            );
        }
        HelpTab::TechnicalInfo => {
            ui.heading("Technical Information");
            ui.label(format!("ShaderCraft core engine version {APP_VERSION}."));
            ui.add_space(6.0);
            egui::Grid::new("help_params_grid")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Parameter");
                    ui.strong("Range");
                    ui.strong("Default");
                    ui.end_row();
                    for kind in NODE_KINDS {
                        for (key, min, max, default) in param_specs(*kind) {
                            ui.label(format!("{}::{key}", kind.as_str()));
                            ui.label(format!("{min} to {max}"));
                            ui.label(format!("{default}"));
                            ui.end_row();
                        }
                    }
                });
        }
        HelpTab::AssistantHelp => {
            ui.heading("Workbench 5: Assistant and Help");
            ui.label(
                "The assistant answers from the live engine state: ask it which graph is active or what the current preview statistics are. No model is selected by default; connect a local model to co-design textures in words.",
            );
        }
    }
}
