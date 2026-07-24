mod settings;

use cir::diagnostic::ValidationReport;
use cir2cvn::generation_nl::{self, GenerationResult};
use cir2cvn::repair::llm::{RepairOutcome, RepairRound, RepairSession};
use cir2cvn::{translate, TranslateError};
use cir2cvn::{VerificationConfig, VerificationResult};
use cvn::analysis::SearchStrategy;
use cvn::export::to_dot;
use serde::Serialize;
use settings::{default_settings, load_settings, save_settings, GuiSettings};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateOk {
    pub cvn_dot: String,
    pub translate_warnings: Vec<String>,
}

/// Unified translate response for the webview (always returned as `Ok`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateCirResponse {
    pub success: bool,
    pub cvn_dot: Option<String>,
    pub translate_warnings: Option<Vec<String>>,
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairResponse {
    pub status: String,
    pub rounds: usize,
    pub cir_json: Option<String>,
    pub last_report: Option<String>,
    pub history: Vec<RepairRound>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfigStatus {
    pub path: String,
    pub exists: bool,
    pub valid: bool,
    pub error: Option<String>,
}

fn parse_strategy(s: &str) -> SearchStrategy {
    match s.to_lowercase().as_str() {
        "dfs" => SearchStrategy::Dfs,
        _ => SearchStrategy::Bfs,
    }
}

async fn llm_client_from_settings(s: &GuiSettings) -> Result<uni_llm::UniLlmClient, String> {
    let path = settings::resolve_config_path(&s.llm_config_path);
    let c = uni_llm::UniLlmClient::from_config(&path)
        .await
        .map_err(|e| e.to_string())?;
    let mut out = c;
    if let Some(ref p) = s.provider_override {
        out = out.with_provider(p);
    }
    if let Some(ref m) = s.model_override {
        out = out.with_model(m);
    }
    Ok(out)
}

#[tauri::command]
fn validate_cir(cir_json: String) -> Result<ValidationReport, String> {
    let program: cir::ast::Program =
        serde_json::from_str(&cir_json).map_err(|e| format!("JSON parse: {e}"))?;
    Ok(cir::validate::validate(&program))
}

#[tauri::command]
fn translate_cir(cir_json: String) -> TranslateCirResponse {
    let program: cir::ast::Program = match serde_json::from_str(&cir_json) {
        Ok(p) => p,
        Err(e) => {
            return TranslateCirResponse {
                success: false,
                cvn_dot: None,
                translate_warnings: None,
                errors: Some(vec![format!("JSON parse: {e}")]),
            };
        }
    };
    match translate(&program) {
        Ok(net) => {
            let warnings = cir2cvn::validate::check_translation(&net);
            TranslateCirResponse {
                success: true,
                cvn_dot: Some(to_dot(&net)),
                translate_warnings: Some(warnings),
                errors: None,
            }
        }
        Err(errs) => TranslateCirResponse {
            success: false,
            cvn_dot: None,
            translate_warnings: None,
            errors: Some(errs.iter().map(TranslateError::to_string).collect()),
        },
    }
}

#[tauri::command]
fn analyze_cvn(
    cir_json: String,
    strategy: String,
    max_states: usize,
) -> Result<VerificationResult, String> {
    let program: cir::ast::Program =
        serde_json::from_str(&cir_json).map_err(|e| format!("JSON parse: {e}"))?;
    let config = VerificationConfig {
        strategy: parse_strategy(&strategy),
        max_states: max_states.max(1),
        ..VerificationConfig::default()
    };
    Ok(cir2cvn::verify_program(&program, &config))
}

#[tauri::command]
async fn repair_cir(cir_json: String, settings: GuiSettings) -> Result<RepairResponse, String> {
    let program: cir::ast::Program =
        serde_json::from_str(&cir_json).map_err(|e| format!("JSON parse: {e}"))?;
    let path = settings::resolve_config_path(&settings.llm_config_path);
    let path_str = path.to_str().ok_or("LLM config path is not valid UTF-8")?;
    let mut client = uni_llm::UniLlmClient::from_config(path_str)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(ref p) = settings.provider_override {
        client = client.with_provider(p);
    }
    if let Some(ref m) = settings.model_override {
        client = client.with_model(m);
    }
    let verification_config = VerificationConfig {
        strategy: parse_strategy(&settings.analysis_strategy),
        max_states: settings.analysis_max_states.max(1),
        ..VerificationConfig::default()
    };
    let session = RepairSession::new(client, settings.repair_max_rounds)
        .with_verification_config(verification_config);
    let outcome = session.repair_loop(&program).await.map_err(|e| e.to_string())?;
    Ok(match outcome {
        RepairOutcome::Fixed {
            fixed_cir_json,
            rounds,
            history,
        } => RepairResponse {
            status: "fixed".into(),
            rounds,
            cir_json: Some(fixed_cir_json),
            last_report: None,
            history,
        },
        RepairOutcome::GaveUp {
            rounds,
            last_report,
            history,
        } => RepairResponse {
            status: "gave_up".into(),
            rounds,
            cir_json: None,
            last_report: Some(last_report),
            history,
        },
    })
}

#[tauri::command]
async fn generate_cir_nl(
    requirements: String,
    settings: GuiSettings,
) -> Result<GenerationResult, String> {
    let client = llm_client_from_settings(&settings).await?;
    let sys = settings
        .nl_system_prompt_override
        .as_deref()
        .filter(|s| !s.trim().is_empty());
    let verification_config = VerificationConfig {
        strategy: parse_strategy(&settings.analysis_strategy),
        max_states: settings.analysis_max_states.max(1),
        ..VerificationConfig::default()
    };
    generation_nl::generate_cir_from_requirements_with_config(
        &client,
        &requirements,
        sys,
        settings.generation_max_rounds.max(1),
        &verification_config,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<GuiSettings, String> {
    load_settings(&app)
}

#[tauri::command]
fn set_settings(app: AppHandle, value: GuiSettings) -> Result<(), String> {
    save_settings(&app, &value)
}

#[tauri::command]
fn reset_settings(app: AppHandle) -> Result<GuiSettings, String> {
    let d = default_settings();
    save_settings(&app, &d)?;
    Ok(d)
}

#[tauri::command]
async fn pick_llm_config_file(app: AppHandle) -> Result<Option<String>, String> {
    let path = app
        .dialog()
        .file()
        .add_filter("TOML", &["toml"])
        .blocking_pick_file();
    Ok(path.and_then(|p| {
        p.into_path()
            .ok()
            .map(|pb| pb.to_string_lossy().into_owned())
    }))
}

#[tauri::command]
async fn test_llm_config(settings: GuiSettings) -> Result<LlmConfigStatus, String> {
    let path = settings::resolve_config_path(&settings.llm_config_path);
    let path_text = path.to_string_lossy().into_owned();
    if !path.is_file() {
        return Ok(LlmConfigStatus {
            path: path_text,
            exists: false,
            valid: false,
            error: Some("configuration file does not exist".into()),
        });
    }
    match llm_client_from_settings(&settings).await {
        Ok(_) => Ok(LlmConfigStatus {
            path: path_text,
            exists: true,
            valid: true,
            error: None,
        }),
        Err(error) => Ok(LlmConfigStatus {
            path: path_text,
            exists: true,
            valid: false,
            error: Some(error),
        }),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            validate_cir,
            translate_cir,
            analyze_cvn,
            repair_cir,
            generate_cir_nl,
            get_settings,
            set_settings,
            reset_settings,
            pick_llm_config_file,
            test_llm_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CPN GUI");
}
