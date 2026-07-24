//! Natural-language requirements → CIR JSON using `uni-llm` and `ceir` validation.

use cir::diagnostic::{Severity, ValidationReport};
use serde::Serialize;
use std::time::Instant;

use crate::llm_common::extract_json_from_llm_response;
use crate::verification::{verify_program, VerificationConfig, VerificationStatus};

const DEFAULT_SYSTEM_PROMPT: &str = include_str!("generation_nl_prompt.md");

/// Default NL→CIR system prompt (same as bundled `generation_nl_prompt.md`).
pub fn default_nl_system_prompt() -> &'static str {
    DEFAULT_SYSTEM_PROMPT
}

/// One round of the generation loop (for UI / logging).
#[derive(Debug, Clone, Serialize)]
pub struct GenerationRoundLog {
    pub round: usize,
    pub candidate_json: Option<String>,
    pub parse_error: Option<String>,
    pub validation_messages: Vec<String>,
    pub verification_status: Option<VerificationStatus>,
    pub verification_messages: Vec<String>,
    pub state_count: usize,
    pub analysis_complete: bool,
    pub accepted: bool,
    pub duration_ms: f64,
}

/// Successful NL generation result.
#[derive(Debug, Clone, Serialize)]
pub struct GenerationResult {
    pub cir_json: String,
    pub rounds: Vec<GenerationRoundLog>,
}

/// Errors that abort generation.
#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("exhausted {0} rounds without valid CIR")]
    Exhausted(usize),
}

fn report_messages(report: &ValidationReport) -> Vec<String> {
    report
        .diagnostics
        .iter()
        .map(|d| {
            let sev = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            if let Some(ref p) = d.path {
                format!("[{}] {} {} — {}", sev, d.code, p, d.message)
            } else {
                format!("[{}] {} — {}", sev, d.code, d.message)
            }
        })
        .collect()
}

/// Run multi-round NL → CIR and require a complete behavioral verification.
pub async fn generate_cir_from_requirements(
    client: &uni_llm::UniLlmClient,
    user_requirements: &str,
    system_prompt: Option<&str>,
    max_rounds: usize,
) -> Result<GenerationResult, GenerationError> {
    generate_cir_from_requirements_with_config(
        client,
        user_requirements,
        system_prompt,
        max_rounds,
        &VerificationConfig::default(),
    )
    .await
}

/// Variant of [`generate_cir_from_requirements`] with explicit verification settings.
pub async fn generate_cir_from_requirements_with_config(
    client: &uni_llm::UniLlmClient,
    user_requirements: &str,
    system_prompt: Option<&str>,
    max_rounds: usize,
    verification_config: &VerificationConfig,
) -> Result<GenerationResult, GenerationError> {
    let system = system_prompt.unwrap_or(DEFAULT_SYSTEM_PROMPT);
    let mut rounds_log = Vec::new();
    let mut user_prompt = format!(
        "Model the following concurrent system as CIR JSON.\n\n### Requirements\n\n{user_requirements}\n"
    );

    for round in 1..=max_rounds {
        let round_start = Instant::now();
        let response = client
            .chat(vec![
                uni_llm::Message::system(system),
                uni_llm::Message::user(&user_prompt),
            ])
            .await
            .map_err(|e| GenerationError::Llm(e.to_string()))?;

        let raw_json = extract_json_from_llm_response(&response.content);

        let program: cir::ast::Program = match serde_json::from_str(&raw_json) {
            Ok(p) => p,
            Err(e) => {
                let msg = e.to_string();
                rounds_log.push(GenerationRoundLog {
                    round,
                    candidate_json: Some(raw_json.clone()),
                    parse_error: Some(msg.clone()),
                    validation_messages: vec![],
                    verification_status: None,
                    verification_messages: vec![],
                    state_count: 0,
                    analysis_complete: false,
                    accepted: false,
                    duration_ms: elapsed_ms(round_start),
                });
                user_prompt = format!(
                    "The previous answer was not valid JSON for a CIR program.\n\
                     JSON parse error: {msg}\n\n\
                     Reply with **only** a corrected complete CIR JSON object.\n\n\
                     Previous text (extracted):\n```json\n{raw_json}\n```"
                );
                continue;
            }
        };

        let report = cir::validate::validate(&program);
        if !report.valid {
            let validation_messages = report_messages(&report);
            rounds_log.push(GenerationRoundLog {
                round,
                candidate_json: Some(raw_json.clone()),
                parse_error: None,
                validation_messages: validation_messages.clone(),
                verification_status: Some(VerificationStatus::InvalidModel),
                verification_messages: validation_messages.clone(),
                state_count: 0,
                analysis_complete: false,
                accepted: false,
                duration_ms: elapsed_ms(round_start),
            });
            let joined = validation_messages.join("\n");
            user_prompt = format!(
                "The CIR JSON has validation errors. Fix them and output **only** the full corrected CIR JSON.\n\n\
                 Errors:\n{joined}\n\n\
                 Current CIR:\n```json\n{raw_json}\n```"
            );
            continue;
        }

        let verification = verify_program(&program, verification_config);
        let verification_messages = verification_messages(&verification);
        let accepted = verification.status == VerificationStatus::VerifiedSafe;
        rounds_log.push(GenerationRoundLog {
            round,
            candidate_json: Some(raw_json.clone()),
            parse_error: None,
            validation_messages: report_messages(&report),
            verification_status: Some(verification.status),
            verification_messages: verification_messages.clone(),
            state_count: verification.state_count,
            analysis_complete: verification.analysis_complete,
            accepted,
            duration_ms: elapsed_ms(round_start),
        });

        if accepted {
            let pretty = serde_json::to_string_pretty(&program)
                .map_err(|e| GenerationError::Llm(e.to_string()))?;
            return Ok(GenerationResult {
                cir_json: pretty,
                rounds: rounds_log,
            });
        }

        let joined = verification_messages.join("\n");
        user_prompt = format!(
            "The CIR JSON is syntactically valid but did not pass complete behavioral verification. Fix the issues and output **only** the full corrected CIR JSON.\n\n\
             Verification findings:\n{joined}\n\n\
             Current CIR:\n```json\n{raw_json}\n```"
        );
    }

    Err(GenerationError::Exhausted(max_rounds))
}

fn verification_messages(result: &crate::verification::VerificationResult) -> Vec<String> {
    let mut messages = Vec::new();
    messages.extend(result.translation_errors.iter().map(|e| format!("translation: {e}")));
    messages.extend(result.translation_warnings.iter().map(|e| format!("translation warning: {e}")));
    if let Some(error) = &result.analysis_error {
        messages.push(format!("analysis: {error}"));
    }
    messages.extend(result.bugs.iter().map(|bug| format!("bug: {}", bug.summary)));
    messages.extend(
        result
            .unmet_goals
            .iter()
            .map(|goal| format!("unmet goal {}: {}", goal.goal.id, goal.reason)),
    );
    messages.extend(result.goal_warnings.iter().map(|warning| format!("goal warning: {warning}")));
    if messages.is_empty() && result.status != VerificationStatus::VerifiedSafe {
        messages.push(format!("verification status: {:?}", result.status));
    }
    messages
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
