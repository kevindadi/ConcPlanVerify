//! LLM-based repair loop using `uni-llm`.
//!
//! This module is only compiled when the `llm` feature is enabled.
//! It provides [`RepairSession`] which drives the full repair cycle:
//!
//! ```text
//! buggy CIR → translate → explore → detect bug → render prompt
//!   → LLM generates fix → parse → translate → explore → verify
//!   → repeat up to max_rounds
//! ```

use crate::repair::render::render_repair_prompt;

/// Outcome of a repair attempt.
#[derive(Debug)]
pub enum RepairOutcome {
    /// Successfully repaired: the fixed CIR passes verification.
    Fixed {
        /// The repaired CIR JSON.
        fixed_cir_json: String,
        /// Number of rounds it took.
        rounds: usize,
    },
    /// Gave up after max_rounds without a successful fix.
    GaveUp {
        /// Number of rounds attempted.
        rounds: usize,
        /// The last error/bug report.
        last_report: String,
    },
}

/// Error type for repair session operations.
#[derive(Debug, thiserror::Error)]
pub enum RepairError {
    #[error("LLM client error: {0}")]
    LlmError(String),
    #[error("CIR parse error: {0}")]
    ParseError(String),
    #[error("Translation error: {0}")]
    TranslateError(String),
}

/// A repair session driving the LLM-based fix loop.
pub struct RepairSession {
    client: uni_llm::UniLlmClient,
    max_rounds: usize,
}

impl RepairSession {
    /// Create a new repair session from a uni-llm config file.
    pub async fn from_config(config_path: &str, max_rounds: usize) -> Result<Self, RepairError> {
        let client = uni_llm::UniLlmClient::from_config(config_path)
            .await
            .map_err(|e| RepairError::LlmError(e.to_string()))?;
        Ok(Self { client, max_rounds })
    }

    /// Create a new repair session from an existing UniLlmClient.
    pub fn new(client: uni_llm::UniLlmClient, max_rounds: usize) -> Self {
        Self { client, max_rounds }
    }

    /// Run the full repair loop on a buggy CIR program.
    ///
    /// 1. Translate buggy CIR to CVN
    /// 2. Run state space exploration
    /// 3. If bugs found, render repair prompt and send to LLM
    /// 4. Parse LLM response as CIR JSON
    /// 5. Translate and verify — if clean, return Fixed; otherwise repeat
    pub async fn repair_loop(
        &self,
        buggy_cir: &cir::ast::Program,
    ) -> Result<RepairOutcome, RepairError> {
        let mut current_json = serde_json::to_string_pretty(buggy_cir)
            .map_err(|e| RepairError::ParseError(e.to_string()))?;

        for round in 1..=self.max_rounds {
            let program: cir::ast::Program = serde_json::from_str(&current_json)
                .map_err(|e| RepairError::ParseError(e.to_string()))?;

            let net = crate::translate(&program)
                .map_err(|errs| {
                    let msgs: Vec<String> = errs.iter().map(|e| e.to_string()).collect();
                    RepairError::TranslateError(msgs.join("; "))
                })?;

            let config = cvn::analysis::AnalysisConfig::default();
            let result = cvn::analysis::explore(&net, &config)
                .map_err(|e| RepairError::TranslateError(e.to_string()))?;

            let reports = crate::repair::analyze(&program, &net, &result);

            if reports.is_empty() {
                return Ok(RepairOutcome::Fixed {
                    fixed_cir_json: current_json,
                    rounds: round,
                });
            }

            let report = &reports[0];
            let prompt = render_repair_prompt(report, &current_json);

            let response = self
                .client
                .chat(vec![
                    uni_llm::Message::system(SYSTEM_PROMPT),
                    uni_llm::Message::user(&prompt),
                ])
                .await
                .map_err(|e| RepairError::LlmError(e.to_string()))?;

            current_json = extract_json_from_response(&response.content);
        }

        Ok(RepairOutcome::GaveUp {
            rounds: self.max_rounds,
            last_report: "exceeded max repair rounds".to_string(),
        })
    }
}

const SYSTEM_PROMPT: &str = "\
你是一个并发系统修复专家。你会收到一个包含并发 bug 的 CIR (Concurrency Intermediate Representation) JSON，\
以及由模型检验工具检测到的 bug 报告。\
请根据报告中的修复建议修复 CIR，输出修复后的完整 CIR JSON。\
只输出 JSON，不要添加任何解释文本。";

/// Extract JSON content from an LLM response, handling markdown code blocks.
fn extract_json_from_response(response: &str) -> String {
    let trimmed = response.trim();
    if trimmed.starts_with("```") {
        let without_fence = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        let end = without_fence.rfind("```").unwrap_or(without_fence.len());
        without_fence[..end].trim().to_string()
    } else {
        trimmed.to_string()
    }
}
