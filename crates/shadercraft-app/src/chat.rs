//! Shared fleet LLM chat for ShaderCraft backed by `craft-ai-ui` / `~/.craft/ai_config.json`.
//! An empty model is an error. This module never invents a fallback reply.

use craft_ai_ui::craft_ai_core::CraftAiConfig;

pub fn active_summary() -> (String, bool) {
    let cfg = CraftAiConfig::load();
    let trimmed = cfg.model.trim();
    if trimmed.is_empty() {
        (format!("{} / (none selected)", cfg.provider), false)
    } else {
        (format!("{} / {}", cfg.provider, trimmed), true)
    }
}

pub fn complete_shader_chat(
    workspace_context: &str,
    history: &[(String, String)],
) -> Result<(String, u128), String> {
    let cfg = CraftAiConfig::load();
    if cfg.model.trim().is_empty() {
        return Err(
            "No model selected in ~/.craft/ai_config.json -- open LLM Settings to choose a provider and model."
                .to_string(),
        );
    }
    let system_prompt = format!(
        "You are a concise ShaderCraft procedural texture and PBR material co-designer. \
         Answer strictly using ASCII punctuation (no em dashes). Live engine state:\n{workspace_context}"
    );
    craft_ai_ui::complete_live(&cfg, &system_prompt, history)
}
