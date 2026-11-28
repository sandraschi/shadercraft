//! ShaderCraft L3 Native Desktop Studio (`eframe` / `egui` 0.37).

use crate::{chat, help_docs, subapps};
use craft_ai_ui::LlmSettingsPanel;
use eframe::egui::{self, Color32, RichText};
use shadercraft_core::{
    default_db_path, param_specs, Command, GraphPreset, NodeKind, ShaderEngine, APP_VERSION,
    MAP_NAMES, OUTPUT_PARAM_KEYS,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

const ATTRIBUTION_MD: &str = include_str!("../../../ATTRIBUTION.md");

const PREVIEW_SIZES: &[u32] = &[64, 128, 256];
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLocale {
    En,
    De,
}

impl AppLocale {
    pub fn label(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::De => "Deutsch",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    GraphEditor,
    Preview,
    PbrMaps,
    Presets,
    Chat,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpTab {
    GetStarted,
    GraphNodes,
    PreviewExport,
    PresetsCli,
    TechnicalInfo,
    AssistantHelp,
}

pub struct ShaderCraftApp {
    pub engine: ShaderEngine,
    pub db_path: PathBuf,
    pub page: Page,
    pub subpage: usize,
    pub locale: AppLocale,
    pub dark_mode: bool,
    pub zoom_factor: f32,
    pub status_line: String,
    pub graph_id: String,
    pub new_graph_name: String,
    pub new_node_kind: NodeKind,
    pub new_edge_from: String,
    pub new_edge_to: String,
    pub new_edge_input: u32,
    pub preview_size_idx: usize,
    pub preview_map_idx: usize,
    pub preview_tex: Option<egui::TextureHandle>,
    pub preview_stats: String,
    pub preview_dirty: bool,
    pub pbr_tex: Vec<(String, egui::TextureHandle)>,
    pub pbr_dirty: bool,
    pub chat_input: String,
    pub chat_history: Vec<(String, String)>,
    pub chat_busy: bool,
    chat_rx: Option<Receiver<Result<(String, u128), String>>>,
    pub show_llm_settings: bool,
    pub llm_settings: LlmSettingsPanel,
    pub help_tab: HelpTab,
    pub show_palette: bool,
    pub palette_query: String,
    pub show_shortcuts: bool,
    pub show_licenses: bool,
    pub show_about: bool,
}

impl Default for ShaderCraftApp {
    fn default() -> Self {
        Self::with_db_path(default_db_path())
    }
}

impl ShaderCraftApp {
    pub fn with_db_path(db_path: PathBuf) -> Self {
        let engine = ShaderEngine::open_or_seed(&db_path).unwrap_or_default();
        let graph_id = engine.active_graph.clone();
        Self {
            engine,
            db_path,
            page: Page::GraphEditor,
            subpage: 0,
            locale: AppLocale::En,
            dark_mode: true,
            zoom_factor: 1.0,
            status_line: format!("ShaderCraft v{APP_VERSION} ready"),
            graph_id,
            new_graph_name: String::new(),
            new_node_kind: NodeKind::Fbm,
            new_edge_from: String::new(),
            new_edge_to: String::new(),
            new_edge_input: 0,
            preview_size_idx: 1,
            preview_map_idx: 0,
            preview_tex: None,
            preview_stats: String::new(),
            preview_dirty: true,
            pbr_tex: Vec::new(),
            pbr_dirty: true,
            chat_input: String::new(),
            chat_history: Vec::new(),
            chat_busy: false,
            chat_rx: None,
            show_llm_settings: false,
            llm_settings: LlmSettingsPanel::new("shadercraft"),
            help_tab: HelpTab::GetStarted,
            show_palette: false,
            palette_query: String::new(),
            show_shortcuts: false,
            show_licenses: false,
            show_about: false,
        }
    }

    pub fn goto(&mut self, page: Page, sub: usize) {
        self.page = page;
        self.subpage = sub;
    }

    fn persist(&mut self) {
        let _ = self.engine.save_db(&self.db_path);
    }

    fn mark_dirty(&mut self) {
        self.preview_dirty = true;
        self.pbr_dirty = true;
    }

    fn sync_graph_selection(&mut self) {
        if self.engine.graph(&self.graph_id).is_err() {
            self.graph_id = self.engine.active_graph.clone();
        }
    }

    fn run_command(&mut self, cmd: Command, ok_note: Option<String>) {
        match cmd.dispatch(&mut self.engine) {
            Ok(val) => {
                if let Some(gid) = val.get("graph_id").and_then(serde_json::Value::as_str) {
                    self.graph_id = gid.to_string();
                }
                if let Some(gid) = val.get("created").and_then(serde_json::Value::as_str) {
                    self.graph_id = gid.to_string();
                }
                self.engine.active_graph = self.graph_id.clone();
                self.persist();
                self.mark_dirty();
                self.sync_graph_selection();
                if let Some(note) = ok_note {
                    self.status_line = note;
                }
            }
            Err(e) => {
                self.status_line = format!("Error: {e}");
            }
        }
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        let theme = if self.dark_mode {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        };
        ctx.set_theme(theme);
        let mut style = (*ctx.style_of(theme)).clone();
        if self.dark_mode {
            style.visuals.override_text_color = Some(Color32::from_rgb(241, 245, 249));
            style.visuals.panel_fill = Color32::from_rgb(15, 23, 42);
        } else {
            style.visuals.override_text_color = Some(Color32::from_rgb(15, 23, 42));
            style.visuals.panel_fill = Color32::from_rgb(248, 250, 252);
        }
        ctx.set_style_of(theme, style);
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let zoom_delta = ctx.input(|i| {
            if i.modifiers.ctrl {
                i.smooth_scroll_delta.y
            } else {
                0.0
            }
        });
        if zoom_delta.abs() > 0.1 {
            let step = if zoom_delta > 0.0 { 0.1 } else { -0.1 };
            self.zoom_factor = (self.zoom_factor + step).clamp(0.75, 1.75);
            ctx.set_zoom_factor(self.zoom_factor);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.goto(Page::Help, 0);
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::K)) {
            self.show_palette = !self.show_palette;
        }
    }

    pub fn render_menu_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save Graphs to SQLite").clicked() {
                    self.persist();
                    self.status_line = format!("Saved to {}", self.db_path.display());
                    ui.close();
                }
                if ui.button("Reload Graphs from Disk").clicked() {
                    match ShaderEngine::open_or_seed(&self.db_path) {
                        Ok(eng) => {
                            self.engine = eng;
                            self.sync_graph_selection();
                            self.mark_dirty();
                            self.status_line = "Reloaded graphs from disk".to_string();
                        }
                        Err(e) => {
                            self.status_line = format!("Reload failed: {e}");
                        }
                    }
                    ui.close();
                }
                if ui.button("Reset Brushed Metal Seed").clicked() {
                    self.run_command(
                        Command::LoadPreset {
                            preset: "brushed_metal".to_string(),
                        },
                        Some("Brushed metal preset loaded".to_string()),
                    );
                    ui.close();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Evaluate Preview Stats").clicked() {
                    let gid = self.graph_id.clone();
                    self.run_command(
                        Command::EvalPreviewStats {
                            graph_id: gid,
                            size: 128,
                        },
                        Some("Preview stats evaluated".to_string()),
                    );
                    ui.close();
                }
                if ui.button("Refresh All Previews").clicked() {
                    self.mark_dirty();
                    self.status_line = "Previews marked for refresh".to_string();
                    ui.close();
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button("Dark").clicked() {
                    self.dark_mode = true;
                    ui.close();
                }
                if ui.button("Light").clicked() {
                    self.dark_mode = false;
                    ui.close();
                }
                ui.separator();
                if ui.button("English").clicked() {
                    self.locale = AppLocale::En;
                    ui.close();
                }
                if ui.button("Deutsch").clicked() {
                    self.locale = AppLocale::De;
                    ui.close();
                }
            });
            ui.menu_button("Insert", |ui| {
                for preset in GraphPreset::list() {
                    if ui.button(format!("Preset: {}", preset.as_str())).clicked() {
                        self.run_command(
                            Command::LoadPreset {
                                preset: preset.as_str().to_string(),
                            },
                            Some(format!("Preset {} loaded", preset.as_str())),
                        );
                        ui.close();
                    }
                }
            });
            ui.menu_button("Modify", |ui| {
                if ui.button("Double Normal Strength").clicked() {
                    let gid = self.graph_id.clone();
                    let current = self
                        .engine
                        .graph(&gid)
                        .map(|g| g.normal_strength)
                        .unwrap_or(2.0);
                    self.run_command(
                        Command::SetParam {
                            graph_id: gid,
                            node_id: "output".to_string(),
                            key: "normal_strength".to_string(),
                            value: (current * 2.0).clamp(0.1, 16.0),
                        },
                        Some("Normal strength doubled".to_string()),
                    );
                    ui.close();
                }
                if ui.button("Halve Normal Strength").clicked() {
                    let gid = self.graph_id.clone();
                    let current = self
                        .engine
                        .graph(&gid)
                        .map(|g| g.normal_strength)
                        .unwrap_or(2.0);
                    self.run_command(
                        Command::SetParam {
                            graph_id: gid,
                            node_id: "output".to_string(),
                            key: "normal_strength".to_string(),
                            value: (current * 0.5).clamp(0.1, 16.0),
                        },
                        Some("Normal strength halved".to_string()),
                    );
                    ui.close();
                }
                if ui.button("Set Matte Finish").clicked() {
                    let gid = self.graph_id.clone();
                    self.run_command(
                        Command::SetParam {
                            graph_id: gid,
                            node_id: "output".to_string(),
                            key: "roughness_base".to_string(),
                            value: 0.9,
                        },
                        Some("Matte finish applied".to_string()),
                    );
                    ui.close();
                }
            });
            ui.menu_button("Commands", |ui| {
                if ui.button("Open 2D Preview").clicked() {
                    self.goto(Page::Preview, 0);
                    ui.close();
                }
                if ui.button("Open PBR Maps").clicked() {
                    self.goto(Page::PbrMaps, 0);
                    ui.close();
                }
                if ui.button("Export PBR Set (128px)").clicked() {
                    self.export_pbr_set_file(128);
                    ui.close();
                }
            });
            ui.menu_button("Window", |ui| {
                for (label, p, s) in [
                    ("Graph Editor", Page::GraphEditor, 0),
                    ("2D Preview", Page::Preview, 0),
                    ("PBR Maps", Page::PbrMaps, 0),
                    ("Presets", Page::Presets, 0),
                    ("Chat", Page::Chat, 0),
                    ("Help (F1)", Page::Help, 0),
                ] {
                    if ui.button(label).clicked() {
                        self.goto(p, s);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("LLM Settings...").clicked() {
                    self.show_llm_settings = true;
                    ui.close();
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Command Palette (Ctrl+K)").clicked() {
                    self.show_palette = true;
                    ui.close();
                }
                if ui.button("Keyboard Shortcuts").clicked() {
                    self.show_shortcuts = true;
                    ui.close();
                }
                if ui.button("Documentation (F1)").clicked() {
                    self.goto(Page::Help, 0);
                    ui.close();
                }
                if ui.button("Licenses").clicked() {
                    self.show_licenses = true;
                    ui.close();
                }
                if ui.button("About ShaderCraft").clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let theme_lbl = if self.dark_mode { "Light" } else { "Dark" };
                if ui.button(theme_lbl).clicked() {
                    self.dark_mode = !self.dark_mode;
                }
                let loc_lbl = self.locale.label();
                if ui.button(loc_lbl).clicked() {
                    self.locale = match self.locale {
                        AppLocale::En => AppLocale::De,
                        AppLocale::De => AppLocale::En,
                    };
                }
                let node_count = self
                    .engine
                    .graph(&self.graph_id)
                    .map(|g| g.nodes.len())
                    .unwrap_or(0);
                ui.label(format!(
                    "Graph: {} | Nodes: {} | Preset count: {}",
                    self.graph_id,
                    node_count,
                    GraphPreset::list().len()
                ));
            });
        });
        let _ = ctx;
    }

    fn export_pbr_set_file(&mut self, size: u32) {
        let gid = self.graph_id.clone();
        match self.engine.export_pbr_set(&gid, size) {
            Ok(files) => {
                let dir = std::env::temp_dir().join("shadercraft_export");
                if std::fs::create_dir_all(&dir).is_err() {
                    self.status_line = "Export failed: cannot create export folder".to_string();
                    return;
                }
                let mut count = 0;
                for (name, bytes) in &files {
                    if std::fs::write(dir.join(name), bytes).is_ok() {
                        count += 1;
                    }
                }
                self.status_line = format!(
                    "Exported {count} of {} maps to {}",
                    files.len(),
                    dir.display()
                );
            }
            Err(e) => {
                self.status_line = format!("Export failed: {e}");
            }
        }
    }

    pub fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("ShaderCraft");
        ui.small("Texture & PBR Studio");
        ui.separator();

        for wb in subapps::registry() {
            let subs = wb.subpages();
            let has_active = subs
                .iter()
                .any(|sp| sp.page == self.page && sp.sub == self.subpage);
            egui::CollapsingHeader::new(RichText::new(wb.title(self.locale)).strong())
                .id_salt(wb.id())
                .default_open(has_active || wb.id() == "graph")
                .show(ui, |ui| {
                    for sp in subs {
                        let active = self.page == sp.page && self.subpage == sp.sub;
                        if ui.selectable_label(active, sp.label(self.locale)).clicked() {
                            self.goto(sp.page, sp.sub);
                        }
                    }
                });
        }
    }

    pub fn render_page(&mut self, ui: &mut egui::Ui) {
        match self.page {
            Page::GraphEditor => self.render_graph_editor(ui),
            Page::Preview => {
                let ctx = ui.ctx().clone();
                self.render_preview(&ctx, ui);
            }
            Page::PbrMaps => {
                let ctx = ui.ctx().clone();
                self.render_pbr_maps(&ctx, ui);
            }
            Page::Presets => self.render_presets(ui),
            Page::Chat => self.render_chat(ui),
            Page::Help => self.render_help_view(ui),
        }
    }

    fn graph_combo(&mut self, ui: &mut egui::Ui) {
        let mut ids: Vec<(String, String)> = self
            .engine
            .graphs
            .values()
            .map(|g| (g.id.clone(), g.name.clone()))
            .collect();
        ids.sort();
        let current = self.graph_id.clone();
        egui::ComboBox::from_label("Graph")
            .selected_text(current.as_str())
            .show_ui(ui, |ui| {
                for (id, name) in &ids {
                    if ui
                        .selectable_label(&current == id, format!("{id} ({name})"))
                        .clicked()
                    {
                        self.graph_id = id.clone();
                        self.engine.active_graph = id.clone();
                        self.persist();
                        self.mark_dirty();
                    }
                }
            });
        ui.horizontal(|ui| {
            let name_lbl = ui.label("New graph:");
            ui.text_edit_singleline(&mut self.new_graph_name)
                .labelled_by(name_lbl.id);
            if ui.button("Create").clicked() {
                let name = self.new_graph_name.clone();
                self.new_graph_name.clear();
                self.run_command(
                    Command::CreateGraph { name },
                    Some("Graph created".to_string()),
                );
            }
        });
    }

    fn render_graph_editor(&mut self, ui: &mut egui::Ui) {
        self.sync_graph_selection();
        ui.heading("Graph Editor");
        self.graph_combo(ui);
        ui.separator();
        if self.subpage == 0 {
            self.render_node_list(ui);
        } else {
            self.render_connections(ui);
        }
    }

    fn render_node_list(&mut self, ui: &mut egui::Ui) {
        let gid = self.graph_id.clone();
        let nodes = self
            .engine
            .graph(&gid)
            .map(|g| g.nodes.clone())
            .unwrap_or_default();
        let output = self
            .engine
            .graph(&gid)
            .map(|g| g.output.clone())
            .unwrap_or_default();
        let mut pending: Vec<(String, String, f32)> = Vec::new();
        let mut delete: Option<String> = None;
        for node in &nodes {
            let is_output = node.id == output;
            let title = if is_output {
                format!("{} [{}] (output)", node.id, node.kind.as_str())
            } else {
                format!("{} [{}]", node.id, node.kind.as_str())
            };
            egui::CollapsingHeader::new(RichText::new(title).strong())
                .id_salt(format!("node_{}", node.id))
                .default_open(is_output)
                .show(ui, |ui| {
                    ui.small(node.kind.blurb());
                    let mut keys: Vec<String> = node.params.keys().cloned().collect();
                    keys.sort();
                    for key in keys {
                        let current = node.params.get(&key).copied().unwrap_or(0.0);
                        let (min, max) = param_specs(node.kind)
                            .iter()
                            .find(|(k, _, _, _)| *k == key.as_str())
                            .map(|(_, lo, hi, _)| (*lo, *hi))
                            .unwrap_or((0.0, 1.0));
                        let mut val = current;
                        let slider =
                            ui.add(egui::Slider::new(&mut val, min..=max).text(key.as_str()));
                        if slider.changed() {
                            pending.push((node.id.clone(), key.clone(), val));
                        }
                    }
                    if !is_output && ui.button(format!("Delete {}", node.id)).clicked() {
                        delete = Some(node.id.clone());
                    }
                });
        }
        ui.separator();
        ui.strong("Graph output (PBR derivation)");
        for key in OUTPUT_PARAM_KEYS {
            let current = self
                .engine
                .graph(&gid)
                .map(|g| match *key {
                    "normal_strength" => g.normal_strength,
                    "roughness_base" => g.roughness_base,
                    "roughness_scale" => g.roughness_scale,
                    "metallic_base" => g.metallic_base,
                    _ => g.metallic_scale,
                })
                .unwrap_or(0.0);
            let (min, max) = if *key == "normal_strength" {
                (0.1, 16.0)
            } else {
                (0.0, 1.0)
            };
            let mut val = current;
            if ui
                .add(egui::Slider::new(&mut val, min..=max).text(*key))
                .changed()
            {
                pending.push(("output".to_string(), (*key).to_string(), val));
            }
        }
        for (node_id, key, value) in pending {
            self.run_command(
                Command::SetParam {
                    graph_id: gid.clone(),
                    node_id,
                    key,
                    value,
                },
                None,
            );
        }
        self.status_line = format!("Graph {gid}: {} nodes", nodes.len());
        if let Some(id) = delete {
            self.run_command(
                Command::DeleteNode {
                    graph_id: gid,
                    node_id: id,
                },
                Some("Node deleted".to_string()),
            );
        }
    }

    fn render_connections(&mut self, ui: &mut egui::Ui) {
        let gid = self.graph_id.clone();
        ui.strong("Wires");
        let edges = self
            .engine
            .graph(&gid)
            .map(|g| g.edges.clone())
            .unwrap_or_default();
        egui::Grid::new("wires_grid").striped(true).show(ui, |ui| {
            ui.strong("From");
            ui.strong("To");
            ui.strong("Input");
            ui.end_row();
            for e in &edges {
                ui.label(e.from.clone());
                ui.label(e.to.clone());
                ui.label(format!("{}", e.input));
                ui.end_row();
            }
        });
        ui.separator();
        ui.strong("Add or replace a wire");
        let node_ids: Vec<String> = self
            .engine
            .graph(&gid)
            .map(|g| g.nodes.iter().map(|n| n.id.clone()).collect())
            .unwrap_or_default();
        if self.new_edge_from.is_empty() {
            self.new_edge_from = node_ids.first().cloned().unwrap_or_default();
        }
        if self.new_edge_to.is_empty() {
            self.new_edge_to = node_ids.first().cloned().unwrap_or_default();
        }
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("From")
                .selected_text(self.new_edge_from.clone())
                .show_ui(ui, |ui| {
                    for id in &node_ids {
                        if ui.selectable_label(&self.new_edge_from == id, id).clicked() {
                            self.new_edge_from = id.clone();
                        }
                    }
                });
            ui.label("to");
            egui::ComboBox::from_label("To")
                .selected_text(self.new_edge_to.clone())
                .show_ui(ui, |ui| {
                    for id in &node_ids {
                        if ui.selectable_label(&self.new_edge_to == id, id).clicked() {
                            self.new_edge_to = id.clone();
                        }
                    }
                });
            egui::ComboBox::from_label("Input slot")
                .selected_text(format!("{}", self.new_edge_input))
                .show_ui(ui, |ui| {
                    for slot in [0u32, 1u32] {
                        if ui
                            .selectable_label(self.new_edge_input == slot, format!("{slot}"))
                            .clicked()
                        {
                            self.new_edge_input = slot;
                        }
                    }
                });
            if ui.button("Connect").clicked() {
                let cmd = Command::ConnectNodes {
                    graph_id: gid.clone(),
                    from: self.new_edge_from.clone(),
                    to: self.new_edge_to.clone(),
                    input: self.new_edge_input,
                };
                self.run_command(cmd, Some("Wire connected".to_string()));
            }
        });
        ui.separator();
        ui.strong("Add a node");
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Node kind")
                .selected_text(self.new_node_kind.as_str())
                .show_ui(ui, |ui| {
                    for kind in NODE_KINDS {
                        if ui
                            .selectable_label(self.new_node_kind == *kind, kind.as_str())
                            .clicked()
                        {
                            self.new_node_kind = *kind;
                        }
                    }
                });
            if ui.button("Add node").clicked() {
                let cmd = Command::AddNode {
                    graph_id: gid.clone(),
                    kind: self.new_node_kind.as_str().to_string(),
                    params: HashMap::new(),
                };
                self.run_command(cmd, Some("Node added".to_string()));
            }
        });
        ui.small("New nodes start with default parameters; tune them on the Node List tab.");
    }

    fn refresh_preview(&mut self, ctx: &egui::Context) {
        let gid = self.graph_id.clone();
        let size = PREVIEW_SIZES[self.preview_size_idx.min(PREVIEW_SIZES.len() - 1)];
        let map = MAP_NAMES[self.preview_map_idx.min(MAP_NAMES.len() - 1)];
        match self.engine.render_rgba(&gid, size, map) {
            Ok((w, h, rgba)) => {
                let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                self.preview_tex =
                    Some(ctx.load_texture("shader_preview", img, egui::TextureOptions::NEAREST));
                match self.engine.height_stats(&gid, size) {
                    Ok((min, max, mean)) => {
                        self.preview_stats =
                            format!("{map} {size}px min {min:.3} max {max:.3} mean {mean:.3}");
                    }
                    Err(e) => {
                        self.preview_stats = format!("Stats failed: {e}");
                    }
                }
            }
            Err(e) => {
                self.preview_stats = format!("Render failed: {e}");
            }
        }
        self.preview_dirty = false;
    }

    fn render_preview(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("2D Preview");
        ui.horizontal(|ui| {
            ui.label("Size:");
            for (idx, size) in PREVIEW_SIZES.iter().enumerate() {
                if ui
                    .selectable_label(self.preview_size_idx == idx, format!("{size}"))
                    .clicked()
                {
                    self.preview_size_idx = idx;
                    self.preview_dirty = true;
                }
            }
            ui.label("Map:");
            for (idx, map) in MAP_NAMES.iter().enumerate() {
                if ui
                    .selectable_label(self.preview_map_idx == idx, *map)
                    .clicked()
                {
                    self.preview_map_idx = idx;
                    self.preview_dirty = true;
                }
            }
            if ui.button("Refresh").clicked() {
                self.preview_dirty = true;
            }
        });
        if self.preview_dirty || self.preview_tex.is_none() {
            self.refresh_preview(ctx);
        }
        ui.label(self.preview_stats.clone());
        ui.separator();
        if self.subpage == 1 {
            ui.strong("Statistics and export");
            ui.label(self.preview_stats.clone());
            ui.horizontal(|ui| {
                if ui.button("Export PBR set (256px)").clicked() {
                    self.export_pbr_set_file(256);
                }
                if ui.button("Evaluate stats (64px)").clicked() {
                    let gid = self.graph_id.clone();
                    match self.engine.height_stats(&gid, 64) {
                        Ok((min, max, mean)) => {
                            self.preview_stats =
                                format!("preview 64px min {min:.3} max {max:.3} mean {mean:.3}");
                        }
                        Err(e) => {
                            self.preview_stats = format!("Stats failed: {e}");
                        }
                    }
                }
            });
        } else if let Some(tex) = self.preview_tex.clone() {
            let side = ui.available_width().clamp(128.0, 512.0);
            ui.image((tex.id(), egui::Vec2::new(side, side)));
        }
    }

    fn refresh_pbr(&mut self, ctx: &egui::Context) {
        let gid = self.graph_id.clone();
        let mut tex = Vec::new();
        for map in MAP_NAMES {
            match self.engine.render_rgba(&gid, 128, map) {
                Ok((w, h, rgba)) => {
                    let img =
                        egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                    tex.push((
                        (*map).to_string(),
                        ctx.load_texture(
                            format!("shader_pbr_{map}"),
                            img,
                            egui::TextureOptions::NEAREST,
                        ),
                    ));
                }
                Err(e) => {
                    self.status_line = format!("PBR render failed: {e}");
                    return;
                }
            }
        }
        self.pbr_tex = tex;
        self.pbr_dirty = false;
    }

    fn render_pbr_maps(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("PBR Maps");
        ui.horizontal(|ui| {
            if ui.button("Refresh maps").clicked() {
                self.pbr_dirty = true;
            }
            if ui.button("Export PBR set (256px)").clicked() {
                self.export_pbr_set_file(256);
            }
        });
        if self.pbr_dirty || self.pbr_tex.is_empty() {
            self.refresh_pbr(ctx);
        }
        if self.subpage == 1 {
            ui.separator();
            ui.strong("Map details");
            ui.label("albedo carries the color ramp; height is the raw field;");
            ui.label(
                "normal is the Sobel derivative; roughness and metallic are scalar derivations.",
            );
            ui.label("Tune scalar outputs with the graph output sliders on the Graph Editor tab.");
        }
        ui.separator();
        egui::Grid::new("pbr_grid").show(ui, |ui| {
            let snapshots = self.pbr_tex.clone();
            for (idx, (name, tex)) in snapshots.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.strong(name.clone());
                    ui.image((tex.id(), egui::Vec2::new(192.0, 192.0)));
                });
                if idx % 2 == 1 {
                    ui.end_row();
                }
            }
        });
    }

    fn render_presets(&mut self, ui: &mut egui::Ui) {
        ui.heading("Presets");
        if self.subpage == 1 {
            ui.label("Starter graphs begin with a value noise source into a levels filter.");
            ui.separator();
        }
        for preset in GraphPreset::list() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.strong(preset.as_str());
                        ui.small(preset.blurb());
                        let built = shadercraft_core::build_preset(*preset);
                        ui.small(format!(
                            "{} nodes, {} wires",
                            built.nodes.len(),
                            built.edges.len()
                        ));
                    });
                    if ui.button(format!("Load {}", preset.as_str())).clicked() {
                        self.run_command(
                            Command::LoadPreset {
                                preset: preset.as_str().to_string(),
                            },
                            Some(format!("Preset {} loaded", preset.as_str())),
                        );
                    }
                });
            });
        }
    }

    fn build_chat_context(&mut self) -> String {
        let gid = self.graph_id.clone();
        let graph_count = match Command::Status.dispatch(&mut self.engine) {
            Ok(val) => val
                .get("graph_count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            Err(_) => 0,
        };
        let node_summary = match self.engine.graph(&gid) {
            Ok(g) => {
                let kinds: Vec<String> = g
                    .nodes
                    .iter()
                    .take(12)
                    .map(|n| format!("{}({:?})", n.id, n.kind))
                    .collect();
                format!(
                    "name='{}', nodes={}, edges={}, normal_strength={:.2}, roughness_base={:.2}, list=[{}]",
                    g.name,
                    g.nodes.len(),
                    g.edges.len(),
                    g.normal_strength,
                    g.roughness_base,
                    kinds.join(", ")
                )
            }
            Err(_) => "graph unavailable".to_string(),
        };
        let stats = self
            .engine
            .height_stats(&gid, 64)
            .map(|(min, max, mean)| format!("min={min:.3} max={max:.3} mean={mean:.3}"))
            .unwrap_or_else(|e| format!("unavailable ({e})"));
        format!(
            "Active graph: {gid} ({graph_count} total graphs) | {node_summary} | 64px heightfield: {stats}"
        )
    }

    fn poll_chat(&mut self, ctx: &egui::Context) {
        if self.chat_busy {
            ctx.request_repaint();
        }
        let Some(rx) = self.chat_rx.as_ref() else {
            return;
        };
        if let Ok(result) = rx.try_recv() {
            self.chat_rx = None;
            self.chat_busy = false;
            match result {
                Ok((reply, ms)) => {
                    self.chat_history.push(("assistant".to_string(), reply));
                    self.status_line = format!("LLM reply received in {ms} ms");
                }
                Err(err) => {
                    self.chat_history
                        .push(("error".to_string(), format!("LLM error: {err}")));
                    self.status_line = format!("LLM error: {err}");
                }
            }
        }
    }

    fn render_chat(&mut self, ui: &mut egui::Ui) {
        let (summary, has_model) = chat::active_summary();
        ui.horizontal(|ui| {
            ui.heading("Assistant");
            ui.separator();
            ui.monospace(&summary);
            if ui.button("LLM Settings...").clicked() {
                self.show_llm_settings = true;
            }
            if ui
                .add_enabled(!self.chat_history.is_empty(), egui::Button::new("Clear"))
                .clicked()
            {
                self.chat_history.clear();
            }
            if self.chat_busy {
                ui.spinner();
                ui.label("Thinking...");
            }
        });
        if !has_model {
            let warn_color = if self.dark_mode {
                Color32::from_rgb(248, 113, 113)
            } else {
                Color32::from_rgb(185, 28, 28)
            };
            ui.colored_label(
                warn_color,
                "No model selected in ~/.craft/ai_config.json -- click LLM Settings... to configure Ollama, LM Studio, or a cloud provider.",
            );
        }
        ui.separator();

        let scroll_h = (ui.available_height() - 64.0).max(120.0);
        egui::ScrollArea::vertical()
            .id_salt("chat_scroll")
            .max_height(scroll_h)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.chat_history.is_empty() {
                    ui.label("Ask how to shape noise nodes, tune PBR roughness/metallic maps, or build a custom material.");
                }
                let history = self.chat_history.clone();
                for (role, text) in &history {
                    ui.group(|ui| {
                        let speaker = match role.as_str() {
                            "user" => "You",
                            "assistant" => "Assistant",
                            _ => "Error",
                        };
                        if role == "error" {
                            let err_color = if self.dark_mode {
                                Color32::from_rgb(248, 113, 113)
                            } else {
                                Color32::from_rgb(185, 28, 28)
                            };
                            ui.colored_label(err_color, speaker);
                            ui.colored_label(err_color, text);
                        } else {
                            ui.strong(speaker);
                            ui.label(text);
                        }
                    });
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            let msg_lbl = ui.label("Message:");
            let response = ui
                .add_enabled(
                    !self.chat_busy,
                    egui::TextEdit::singleline(&mut self.chat_input)
                        .desired_width((ui.available_width() - 80.0).max(160.0))
                        .hint_text("Ask about procedural nodes or PBR materials..."),
                )
                .labelled_by(msg_lbl.id);
            let clicked = ui
                .add_enabled(!self.chat_busy, egui::Button::new("Send"))
                .clicked();
            if clicked || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                let question = self.chat_input.trim().to_string();
                if !question.is_empty() {
                    self.chat_input.clear();
                    self.answer_chat(question);
                }
            }
        });
        ui.small("Shared fleet LLM chat (~/.craft/ai_config.json) grounded in live texture engine state.");
    }

    fn answer_chat(&mut self, question: String) {
        if self.chat_busy {
            return;
        }
        self.chat_history.push(("user".to_string(), question));
        let history_for_llm: Vec<(String, String)> = self
            .chat_history
            .iter()
            .filter(|(role, _)| role == "user" || role == "assistant")
            .cloned()
            .collect();
        let workspace_context = self.build_chat_context();
        let (tx, rx) = mpsc::channel();
        self.chat_rx = Some(rx);
        self.chat_busy = true;
        self.status_line = "Waiting for LLM reply...".to_string();
        std::thread::spawn(move || {
            let res = chat::complete_shader_chat(&workspace_context, &history_for_llm);
            let _ = tx.send(res);
        });
    }

    fn render_help_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Help");
        ui.horizontal(|ui| {
            for (tab, label) in [
                (HelpTab::GetStarted, "Get Started"),
                (HelpTab::GraphNodes, "Graph Nodes"),
                (HelpTab::PreviewExport, "Preview & Export"),
                (HelpTab::PresetsCli, "Presets & CLI"),
                (HelpTab::TechnicalInfo, "Technical Info"),
                (HelpTab::AssistantHelp, "Assistant"),
            ] {
                if ui.selectable_label(self.help_tab == tab, label).clicked() {
                    self.help_tab = tab;
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("help_scroll")
            .show(ui, |ui| help_docs::render_help_tab_content(self, ui));
    }

    fn render_modals(&mut self, ctx: &egui::Context) {
        if self.show_palette {
            let mut open = self.show_palette;
            let mut nav_target: Option<(Page, usize)> = None;
            let mut open_llm_settings = false;
            egui::Window::new("Command Palette")
                .open(&mut open)
                .show(ctx, |ui| {
                    let _ = egui::TextEdit::singleline(&mut self.palette_query)
                        .hint_text("Filter commands...")
                        .show(ui);
                    let query = self.palette_query.to_ascii_lowercase();
                    for wb in subapps::registry() {
                        for sp in wb.subpages() {
                            let label =
                                format!("{} / {}", wb.title(self.locale), sp.label(self.locale));
                            if (query.is_empty() || label.to_ascii_lowercase().contains(&query))
                                && ui.button(label.as_str()).clicked()
                            {
                                nav_target = Some((sp.page, sp.sub));
                            }
                        }
                    }
                    if (query.is_empty() || "llm settings".contains(&query))
                        && ui.button("Open LLM Settings").clicked()
                    {
                        open_llm_settings = true;
                    }
                });
            self.show_palette = open;
            if let Some((page, sub)) = nav_target {
                self.goto(page, sub);
                self.show_palette = false;
            }
            if open_llm_settings {
                self.show_llm_settings = true;
                self.show_palette = false;
            }
        }
        if self.show_llm_settings {
            let mut open = self.show_llm_settings;
            egui::Window::new("LLM Settings")
                .collapsible(false)
                .resizable(true)
                .default_width(560.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("shadercraft_llm_settings_scroll")
                        .show(ui, |ui| {
                            self.llm_settings.render(ui, self.dark_mode);
                        });
                });
            self.show_llm_settings = open;
        }
        if self.show_shortcuts {
            egui::Window::new("Keyboard Shortcuts")
                .open(&mut self.show_shortcuts)
                .show(ctx, |ui| {
                    ui.label("F1: open Help");
                    ui.label("Ctrl+K: toggle the Command Palette");
                    ui.label("Ctrl+mouse wheel: zoom the interface");
                });
        }
        if self.show_licenses {
            egui::Window::new("Licenses & Attribution")
                .open(&mut self.show_licenses)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("licenses_scroll")
                        .show(ui, |ui| ui.label(ATTRIBUTION_MD));
                });
        }
        if self.show_about {
            egui::Window::new("About ShaderCraft")
                .open(&mut self.show_about)
                .show(ctx, |ui| {
                    ui.label(format!("ShaderCraft Studio v{APP_VERSION}"));
                    ui.label("Procedural texture forge: node-graph synthesis with PBR map export.");
                });
        }
    }
}

impl eframe::App for ShaderCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_chat(&ctx);
        self.apply_theme(&ctx);
        self.handle_shortcuts(&ctx);

        egui::Panel::top("shadercraft_top_menu").show(ui, |ui| {
            self.render_menu_bar(&ctx, ui);
        });
        egui::Panel::bottom("shadercraft_status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.status_line.clone());
            });
        });
        egui::Panel::left("shadercraft_sidebar")
            .default_size(225.0)
            .show(ui, |ui| {
                self.render_sidebar(ui);
            });
        egui::CentralPanel::default().show(ui, |ui| {
            self.render_page(ui);
        });

        self.render_modals(&ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;

    fn temp_app() -> ShaderCraftApp {
        let mut p = std::env::temp_dir();
        p.push(format!("shadercraft_app_test_{}.db", std::process::id()));
        ShaderCraftApp::with_db_path(p)
    }

    #[test]
    fn subapp_registry_exposes_all_five_workbenches() {
        let reg = subapps::registry();
        assert_eq!(reg.len(), 5);
        let total_subpages: usize = reg.iter().map(|w| w.subpages().len()).sum();
        assert_eq!(total_subpages, 10);
    }

    #[test]
    fn headless_egui_kittest_renders_all_ten_workbench_subpages() {
        let mut app = temp_app();
        for wb in subapps::registry() {
            for sp in wb.subpages() {
                app.goto(sp.page, sp.sub);
                let mut harness = Harness::new_ui(|ui| {
                    let ctx = ui.ctx().clone();
                    app.render_menu_bar(&ctx, ui);
                    app.render_sidebar(ui);
                    app.render_page(ui);
                });
                harness.run();
            }
        }
        let _ = std::fs::remove_file(&app.db_path);
        let _ = app.preview_tex;
        let _ = app.pbr_tex.len();
    }

    #[test]
    fn headless_egui_kittest_llm_settings_renders() {
        let mut app = temp_app();
        let mut harness = egui_kittest::HarnessBuilder::default()
            .with_accessibility_check(false)
            .build_ui(|ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    app.llm_settings.render(ui, app.dark_mode);
                });
            });
        harness.run();
        let _ = std::fs::remove_file(&app.db_path);
    }
}
