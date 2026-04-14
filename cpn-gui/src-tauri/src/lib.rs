mod settings;

use cir::diagnostic::ValidationReport;
use cir2cvn::generation_nl::{self, GenerationResult};
use cir2cvn::repair::llm::{RepairOutcome, RepairSession};
use cir2cvn::{translate, TranslateError};
use cvn::analysis::{explore, AnalysisConfig, SearchStrategy};
use cvn::export::to_dot;
use serde::{Deserialize, Serialize};
use settings::{default_settings, load_settings, save_settings, GuiSettings};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateOk {
    pub cvn_dot: String,
    pub translate_warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateErr {
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiringStepDto {
    pub transition_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_sids: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadlockSummary {
    pub kind: String,
    pub trace: Vec<FiringStepDto>,
    pub trace_len: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeResponse {
    pub translate_warnings: Vec<String>,
    pub cvn_dot: String,
    pub state_count: usize,
    pub deadlock_count: usize,
    pub deadlocks: Vec<DeadlockSummary>,
    pub explore_error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairResponse {
    pub status: String,
    pub rounds: usize,
    pub cir_json: Option<String>,
    pub last_report: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub requirements: String,
    pub settings: GuiSettings,
}

fn parse_strategy(s: &str) -> SearchStrategy {
    match s.to_lowercase().as_str() {
        "dfs" => SearchStrategy::Dfs,
        _ => SearchStrategy::Bfs,
    }
}

async fn llm_client_from_settings(s: &GuiSettings) -> Result<uni_llm::UniLlmClient, String> {
    let path = settings::expand_config_path(&s.llm_config_path);
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
fn translate_cir(cir_json: String) -> Result<TranslateOk, TranslateErr> {
    let program: cir::ast::Program = match serde_json::from_str(&cir_json) {
        Ok(p) => p,
        Err(e) => {
            return Err(TranslateErr {
                errors: vec![format!("JSON parse: {e}")],
            });
        }
    };
    match translate(&program) {
        Ok(net) => {
            let warnings = cir2cvn::validate::check_translation(&net);
            Ok(TranslateOk {
                cvn_dot: to_dot(&net),
                translate_warnings: warnings,
            })
        }
        Err(errs) => Err(TranslateErr {
            errors: errs.iter().map(TranslateError::to_string).collect(),
        }),
    }
}

#[tauri::command]
fn analyze_cvn(
    cir_json: String,
    strategy: String,
    max_states: usize,
) -> Result<AnalyzeResponse, String> {
    let program: cir::ast::Program =
        serde_json::from_str(&cir_json).map_err(|e| format!("JSON parse: {e}"))?;
    let net = translate(&program).map_err(|errs| {
        errs
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let translate_warnings = cir2cvn::validate::check_translation(&net);
    let cvn_dot = to_dot(&net);
    let config = AnalysisConfig {
        strategy: parse_strategy(&strategy),
        max_states,
    };
    let explored = explore(&net, &config);
    match explored {
        Ok(result) => {
            let deadlocks: Vec<DeadlockSummary> = result
                .deadlocks
                .iter()
                .map(|cx| DeadlockSummary {
                    kind: format!("{:?}", cx.kind),
                    trace: cx
                        .trace
                        .iter()
                        .map(|step| FiringStepDto {
                            transition_id: step.transition_id.0.clone(),
                            anchor_sids: {
                                #[cfg(feature = "cir-anchor")]
                                {
                                    let v: Vec<String> =
                                        step.anchor_sids.iter().cloned().collect();
                                    if v.is_empty() {
                                        None
                                    } else {
                                        Some(v)
                                    }
                                }
                                #[cfg(not(feature = "cir-anchor"))]
                                {
                                    None
                                }
                            },
                        })
                        .collect(),
                    trace_len: cx.trace.len(),
                })
                .collect();
            Ok(AnalyzeResponse {
                translate_warnings,
                cvn_dot,
                state_count: result.state_count,
                deadlock_count: deadlocks.len(),
                deadlocks,
                explore_error: None,
            })
        }
        Err(e) => Ok(AnalyzeResponse {
            translate_warnings,
            cvn_dot,
            state_count: 0,
            deadlock_count: 0,
            deadlocks: vec![],
            explore_error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
async fn repair_cir(cir_json: String, settings: GuiSettings) -> Result<RepairResponse, String> {
    let program: cir::ast::Program =
        serde_json::from_str(&cir_json).map_err(|e| format!("JSON parse: {e}"))?;
    let path = settings::expand_config_path(&settings.llm_config_path);
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
    let session = RepairSession::new(client, settings.repair_max_rounds);
    let outcome = session.repair_loop(&program).await.map_err(|e| e.to_string())?;
    Ok(match outcome {
        RepairOutcome::Fixed {
            fixed_cir_json,
            rounds,
        } => RepairResponse {
            status: "fixed".into(),
            rounds,
            cir_json: Some(fixed_cir_json),
            last_report: None,
        },
        RepairOutcome::GaveUp { rounds, last_report } => RepairResponse {
            status: "gave_up".into(),
            rounds,
            cir_json: None,
            last_report: Some(last_report),
        },
    })
}

#[tauri::command]
async fn generate_cir_nl(req: GenerateRequest) -> Result<GenerationResult, String> {
    let client = llm_client_from_settings(&req.settings).await?;
    let sys = req
        .settings
        .nl_system_prompt_override
        .as_deref()
        .filter(|s| !s.trim().is_empty());
    generation_nl::generate_cir_from_requirements(
        &client,
        &req.requirements,
        sys,
        req.settings.generation_max_rounds.max(1),
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
    Ok(path.map(|p| p.to_string()))
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running CPN GUI");
}
