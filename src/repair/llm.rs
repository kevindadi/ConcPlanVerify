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

use crate::llm_common::extract_json_from_llm_response;
use crate::repair::render::{render_goal_repair_prompt, render_repair_prompt};

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
    /// The loop implements Algorithm 1 from the paper:
    /// 1. Parse CIR JSON
    /// 2. Run post-translation static checks
    /// 3. Translate CIR to CVN
    /// 4. Run state-space exploration and bug detection
    /// 5. Check business goal reachability (if bug-free)
    /// 6. If bugs found or goals unreachable, render prompt and query LLM
    /// 7. Parse response, repeat up to max_rounds
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

            // Layer 1: post-translation static checks
            let static_warnings = crate::validate::check_translation(&net);
            if !static_warnings.is_empty() {
                eprintln!(
                    "Round {round}: {} static check warnings: {}",
                    static_warnings.len(),
                    static_warnings.join("; ")
                );
            }

            // Layer 2: state-space exploration + bug detection
            let config = cvn::analysis::AnalysisConfig::default();
            let result = cvn::analysis::explore(&net, &config)
                .map_err(|e| RepairError::TranslateError(e.to_string()))?;

            let reports = crate::repair::analyze(&program, &net, &result);

            if reports.is_empty() {
                // Layer 3: goal reachability check (only when bug-free)
                let goal_failures = check_business_goals(&program, &net)
                    .map_err(|e| RepairError::TranslateError(e))?;
                if goal_failures.is_empty() {
                    return Ok(RepairOutcome::Fixed {
                        fixed_cir_json: current_json,
                        rounds: round,
                    });
                }

                let goal_prompt =
                    render_goal_repair_prompt(&program, &goal_failures, &current_json);

                let response = self
                    .client
                    .chat(vec![
                        uni_llm::Message::system(SYSTEM_PROMPT),
                        uni_llm::Message::user(&goal_prompt),
                    ])
                    .await
                    .map_err(|e| RepairError::LlmError(e.to_string()))?;

                current_json = extract_json_from_llm_response(&response.content);
                continue;
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

            current_json = extract_json_from_llm_response(&response.content);
        }

        Ok(RepairOutcome::GaveUp {
            rounds: self.max_rounds,
            last_report: "exceeded max repair rounds".to_string(),
        })
    }
}

const SYSTEM_PROMPT: &str = include_str!("cir_schema_prompt.md");

/// Check business goals against the CVN reachability graph and return
/// the goals that were not witnessed in any reachable state.
///
/// Translation is performed by [`crate::translate_goals`] (which maps
/// user-level resource/function references to CVN place IDs and also
/// repairs the Channel/Condvar "availability" semantics) and the
/// reachability query itself is performed by [`cvn::analysis::check_goals`].
///
/// Returns a textual [`RepairError`] payload if either step fails.
pub(crate) fn check_business_goals(
    program: &cir::ast::Program,
    net: &cvn::net::CvnNet,
) -> Result<Vec<cvn::analysis::UnmetGoal>, String> {
    if program.goals.is_empty() {
        return Ok(Vec::new());
    }

    let (specs, warnings) = crate::translate_goals(program);
    for w in &warnings {
        eprintln!("goal translation warning: {w}");
    }

    cvn::analysis::check_goals(net, &specs, &cvn::analysis::AnalysisConfig::default())
        .map_err(|e| e.to_string())
}
