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
use crate::verification::{verify_program, VerificationConfig, VerificationResult, VerificationStatus};
use serde::Serialize;
use std::time::Instant;

/// Structured record for one candidate checked by the repair loop.
#[derive(Debug, Clone, Serialize)]
pub struct RepairRound {
    pub round: usize,
    pub candidate_cir_json: Option<String>,
    pub parse_error: Option<String>,
    pub verification: Option<VerificationResult>,
    pub accepted: bool,
    pub rejection_reason: Option<String>,
    pub duration_ms: f64,
}

/// Outcome of a repair attempt.
#[derive(Debug)]
pub enum RepairOutcome {
    /// Successfully repaired: the fixed CIR passes verification.
    Fixed {
        /// The repaired CIR JSON.
        fixed_cir_json: String,
        /// Number of rounds it took.
        rounds: usize,
        /// Complete candidate/verification history.
        history: Vec<RepairRound>,
    },
    /// Gave up after max_rounds without a successful fix.
    GaveUp {
        /// Number of rounds attempted.
        rounds: usize,
        /// The last error/bug report.
        last_report: String,
        /// Complete candidate/verification history.
        history: Vec<RepairRound>,
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
    verification_config: VerificationConfig,
}

impl RepairSession {
    /// Create a new repair session from a uni-llm config file.
    pub async fn from_config(config_path: &str, max_rounds: usize) -> Result<Self, RepairError> {
        let client = uni_llm::UniLlmClient::from_config(config_path)
            .await
            .map_err(|e| RepairError::LlmError(e.to_string()))?;
        Ok(Self {
            client,
            max_rounds,
            verification_config: VerificationConfig::default(),
        })
    }

    /// Create a new repair session from an existing UniLlmClient.
    pub fn new(client: uni_llm::UniLlmClient, max_rounds: usize) -> Self {
        Self {
            client,
            max_rounds,
            verification_config: VerificationConfig::default(),
        }
    }

    /// Configure the verification pipeline used for every repair candidate.
    pub fn with_verification_config(mut self, config: VerificationConfig) -> Self {
        self.verification_config = config;
        self
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
        let mut history = Vec::new();
        let mut last_report = "no candidate was checked".to_string();

        for round in 1..=self.max_rounds {
            let round_start = Instant::now();
            let program: cir::ast::Program = serde_json::from_str(&current_json)
                .map_err(|e| {
                    let message = e.to_string();
                    history.push(RepairRound {
                        round,
                        candidate_cir_json: Some(current_json.clone()),
                        parse_error: Some(message.clone()),
                        verification: None,
                        accepted: false,
                        rejection_reason: Some(message.clone()),
                        duration_ms: elapsed_ms(round_start),
                    });
                    RepairError::ParseError(message)
                })?;

            let verification = verify_program(&program, &self.verification_config);
            let accepted = verification.status == VerificationStatus::VerifiedSafe;
            let rejection_reason = verification_reason(&verification);
            last_report = rejection_reason
                .clone()
                .unwrap_or_else(|| format!("status: {:?}", verification.status));
            history.push(RepairRound {
                round,
                candidate_cir_json: Some(current_json.clone()),
                parse_error: None,
                verification: Some(verification.clone()),
                accepted,
                rejection_reason: rejection_reason.clone(),
                duration_ms: elapsed_ms(round_start),
            });

            if accepted {
                return Ok(RepairOutcome::Fixed {
                    fixed_cir_json: current_json,
                    rounds: round,
                    history,
                });
            }

            let prompt = if !verification.bugs.is_empty() {
                verification
                    .bugs
                    .iter()
                    .map(|report| render_repair_prompt(report, &current_json))
                    .collect::<Vec<_>>()
                    .join("\n\n--- NEXT BUG ---\n\n")
            } else if !verification.unmet_goals.is_empty() {
                render_goal_repair_prompt(&program, &verification.unmet_goals, &current_json)
            } else {
                format!(
                    "# CIR Verification Repair Request\n\n{}\n\n## Current CIR\n```json\n{}\n```\n\nOutput the complete revised CIR JSON only.",
                    rejection_reason.unwrap_or_else(|| "The candidate did not pass verification.".into()),
                    current_json
                )
            };

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
            last_report: format!("exceeded max repair rounds; {last_report}"),
            history,
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

fn verification_reason(result: &VerificationResult) -> Option<String> {
    let mut reasons = Vec::new();
    reasons.extend(result.translation_errors.iter().map(|e| format!("translation: {e}")));
    if let Some(error) = &result.analysis_error {
        reasons.push(format!("analysis: {error}"));
    }
    reasons.extend(result.bugs.iter().map(|bug| format!("bug: {}", bug.summary)));
    reasons.extend(
        result
            .unmet_goals
            .iter()
            .map(|goal| format!("goal {}: {}", goal.goal.id, goal.reason)),
    );
    reasons.extend(result.goal_warnings.iter().map(|warning| format!("goal warning: {warning}")));
    if reasons.is_empty() && result.status != VerificationStatus::VerifiedSafe {
        reasons.push(format!("verification status: {:?}", result.status));
    }
    if reasons.is_empty() {
        None
    } else {
        Some(reasons.join("\n"))
    }
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
