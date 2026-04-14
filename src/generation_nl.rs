//! Natural-language requirements → CIR JSON using `uni-llm` and `ceir` validation.

use cir::diagnostic::{Severity, ValidationReport};
use serde::Serialize;

use crate::llm_common::extract_json_from_llm_response;

const DEFAULT_SYSTEM_PROMPT: &str = include_str!("generation_nl_prompt.md");

/// Default NL→CIR system prompt (same as bundled `generation_nl_prompt.md`).
pub fn default_nl_system_prompt() -> &'static str {
    DEFAULT_SYSTEM_PROMPT
}

/// One round of the generation loop (for UI / logging).
#[derive(Debug, Clone, Serialize)]
pub struct GenerationRoundLog {
    pub round: usize,
    pub parse_error: Option<String>,
    pub validation_messages: Vec<String>,
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

/// Run multi-round NL → CIR: LLM produces JSON, then parse + `ceir::validate`; on failure, feed errors back to the model.
pub async fn generate_cir_from_requirements(
    client: &uni_llm::UniLlmClient,
    user_requirements: &str,
    system_prompt: Option<&str>,
    max_rounds: usize,
) -> Result<GenerationResult, GenerationError> {
    let system = system_prompt.unwrap_or(DEFAULT_SYSTEM_PROMPT);
    let mut rounds_log = Vec::new();
    let mut user_prompt = format!(
        "Model the following concurrent system as CIR JSON.\n\n### Requirements\n\n{user_requirements}\n"
    );

    for round in 1..=max_rounds {
        let response = client
            .chat(vec![
                uni_llm::Message::system(system),
                uni_llm::Message::user(&user_prompt),
            ])
            .await
            .map_err(|e| GenerationError::Llm(e.to_string()))?;

        let raw_json = extract_json_from_llm_response(&response.content);

        let program: Result<cir::ast::Program, _> = serde_json::from_str(&raw_json);
        let (parse_error, program) = match program {
            Ok(p) => (None, p),
            Err(e) => {
                let msg = e.to_string();
                rounds_log.push(GenerationRoundLog {
                    round,
                    parse_error: Some(msg.clone()),
                    validation_messages: vec![],
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
        if report.valid {
            let pretty = serde_json::to_string_pretty(&program)
                .map_err(|e| GenerationError::Llm(e.to_string()))?;
            rounds_log.push(GenerationRoundLog {
                round,
                parse_error: None,
                validation_messages: vec![],
            });
            return Ok(GenerationResult {
                cir_json: pretty,
                rounds: rounds_log,
            });
        }

        let validation_messages = report_messages(&report);
        rounds_log.push(GenerationRoundLog {
            round,
            parse_error,
            validation_messages: validation_messages.clone(),
        });

        let joined = validation_messages.join("\n");
        user_prompt = format!(
            "The CIR JSON has validation errors. Fix them and output **only** the full corrected CIR JSON.\n\n\
             Errors:\n{joined}\n\n\
             Current CIR:\n```json\n{raw_json}\n```"
        );
    }

    Err(GenerationError::Exhausted(max_rounds))
}
