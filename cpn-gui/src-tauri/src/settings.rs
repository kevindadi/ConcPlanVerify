//! Persisted GUI settings (`app_config_dir/app_settings.json`).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiSettings {
    #[serde(default = "default_llm_path")]
    pub llm_config_path: String,
    #[serde(default)]
    pub provider_override: Option<String>,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default = "default_gen_rounds")]
    pub generation_max_rounds: usize,
    #[serde(default = "default_rep_rounds")]
    pub repair_max_rounds: usize,
    #[serde(default)]
    pub nl_system_prompt_override: Option<String>,
    #[serde(default = "default_strategy")]
    pub analysis_strategy: String,
    #[serde(default = "default_max_states")]
    pub analysis_max_states: usize,
    #[serde(default = "default_render_dot")]
    pub render_dot_preview: bool,
}

fn default_llm_path() -> String {
    "uni-llm.toml".into()
}

fn default_gen_rounds() -> usize {
    8
}
fn default_rep_rounds() -> usize {
    6
}
fn default_strategy() -> String {
    "bfs".into()
}
fn default_max_states() -> usize {
    100_000
}
fn default_render_dot() -> bool {
    true
}

pub fn default_settings() -> GuiSettings {
    GuiSettings {
        llm_config_path: "uni-llm.toml".into(),
        provider_override: None,
        model_override: None,
        generation_max_rounds: 8,
        repair_max_rounds: 6,
        nl_system_prompt_override: None,
        analysis_strategy: "bfs".into(),
        analysis_max_states: 100_000,
        render_dot_preview: true,
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?;
    Ok(dir.join("app_settings.json"))
}

pub fn expand_config_path(p: &str) -> PathBuf {
    let p = p.trim();
    if p.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(&p[2..]);
        }
    }
    PathBuf::from(p)
}

/// Resolve relative config paths against the process-independent app context.
/// Desktop launches may use a different working directory than the shell.
pub fn resolve_config_path(p: &str) -> PathBuf {
    let path = expand_config_path(p);
    if path.is_absolute() {
        path
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(path)
    } else {
        path
    }
}

pub fn load_settings(app: &AppHandle) -> Result<GuiSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(default_settings());
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn save_settings(app: &AppHandle, value: &GuiSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}
