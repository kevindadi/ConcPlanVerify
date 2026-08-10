//! Unified ConcIR verification pipeline.
//!
//! Every user-facing entry point should use this module so that static ConcIR
//! validation, translation sanity checks, state-space exploration, bug
//! reports, and business goals have one consistent contract.

use std::time::Instant;

use concir::diagnostic::ValidationReport;
use serde::Serialize;
use unipn::analysis::{AnalysisConfig, SearchStrategy};
use unipn::model::{ControlSub, PlaceKind};
use unipn::NetLike;

use crate::goals::check_goals;
use crate::repair::{analyze, BugReport};

/// Configuration shared by generation, repair, CLI, and GUI verification.
#[derive(Clone, Debug)]
pub struct VerificationConfig {
    pub strategy: SearchStrategy,
    pub max_states: usize,
    pub analyze_dead_transitions: bool,
    pub check_goals: bool,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            strategy: SearchStrategy::Bfs,
            max_states: 100_000,
            analyze_dead_transitions: true,
            check_goals: true,
        }
    }
}

impl VerificationConfig {
    fn analysis_config(&self) -> AnalysisConfig {
        AnalysisConfig {
            strategy: self.strategy,
            max_states: self.max_states.max(1),
            ..AnalysisConfig::default()
        }
    }
}

/// Overall result of the verification pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    InvalidModel,
    TranslationFailed,
    AnalysisIncomplete,
    VerifiedSafe,
    VerifiedUnsafe,
    GoalsUnmet,
}

/// Stage timings in milliseconds.
#[derive(Clone, Debug, Default, Serialize)]
pub struct VerificationTimings {
    pub validation_ms: f64,
    pub translation_ms: f64,
    pub analysis_ms: f64,
    pub goals_ms: f64,
    pub total_ms: f64,
}

/// Place counts partitioned by [`PlaceKind`].
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct PlacesByKind {
    pub control: usize,
    pub resource: usize,
    pub wait: usize,
}

fn net_size_metrics(net: &unipn::Net) -> (PlacesByKind, usize, usize) {
    let mut by_kind = PlacesByKind::default();
    for place in net.places() {
        match &place.kind {
            PlaceKind::Control(ControlSub::WaitPoint) => by_kind.wait += 1,
            PlaceKind::Control(_) => by_kind.control += 1,
            PlaceKind::Resource(_) => by_kind.resource += 1,
        }
    }
    let input_arcs = net.pre().iter().count();
    let output_arcs = net.post().iter().count();
    (by_kind, input_arcs, output_arcs)
}

/// A complete verification result. The optional fields are populated only
/// when the corresponding stage ran successfully.
#[derive(Clone, Debug, Serialize)]
pub struct VerificationResult {
    pub status: VerificationStatus,
    pub validation: ValidationReport,
    pub translation_errors: Vec<String>,
    pub translation_warnings: Vec<String>,
    pub places: usize,
    pub transitions: usize,
    pub places_by_kind: PlacesByKind,
    pub input_arcs: usize,
    pub output_arcs: usize,
    pub cvn_dot: Option<String>,
    pub state_count: usize,
    pub analysis_complete: bool,
    pub max_states: usize,
    pub analysis_error: Option<String>,
    pub bugs: Vec<BugReport>,
    pub unmet_goals: Vec<crate::goals::UnmetGoal>,
    pub goal_warnings: Vec<String>,
    pub declared_goal_count: usize,
    pub timings: VerificationTimings,
}

impl VerificationResult {
    fn empty(validation: ValidationReport, status: VerificationStatus) -> Self {
        Self {
            status,
            validation,
            translation_errors: Vec::new(),
            translation_warnings: Vec::new(),
            places: 0,
            transitions: 0,
            places_by_kind: PlacesByKind::default(),
            input_arcs: 0,
            output_arcs: 0,
            cvn_dot: None,
            state_count: 0,
            analysis_complete: false,
            max_states: 0,
            analysis_error: None,
            bugs: Vec::new(),
            unmet_goals: Vec::new(),
            goal_warnings: Vec::new(),
            declared_goal_count: 0,
            timings: VerificationTimings::default(),
        }
    }
}

/// Verify a parsed ConcIR program through all available layers.
pub fn verify_program(
    program: &concir::ast::Program,
    config: &VerificationConfig,
) -> VerificationResult {
    let total = Instant::now();
    let validation_start = Instant::now();
    let validation = concir::validate::validate(program);
    let validation_ms = elapsed_ms(validation_start);

    if !validation.valid {
        let mut result = VerificationResult::empty(validation, VerificationStatus::InvalidModel);
        result.declared_goal_count = program.goals.len();
        result.timings.validation_ms = validation_ms;
        result.timings.total_ms = elapsed_ms(total);
        return result;
    }

    let translation_start = Instant::now();
    let net = match crate::translate(program) {
        Ok(net) => net,
        Err(errors) => {
            let mut result =
                VerificationResult::empty(validation, VerificationStatus::TranslationFailed);
            result.translation_errors = errors.iter().map(ToString::to_string).collect();
            result.declared_goal_count = program.goals.len();
            result.timings.validation_ms = validation_ms;
            result.timings.translation_ms = elapsed_ms(translation_start);
            result.timings.total_ms = elapsed_ms(total);
            return result;
        }
    };
    let translation_ms = elapsed_ms(translation_start);

    let analysis_start = Instant::now();
    let analysis = unipn::analysis::explore(&net, &config.analysis_config());
    let analysis_ms = elapsed_ms(analysis_start);
    let (places_by_kind, input_arcs, output_arcs) = net_size_metrics(&net);
    let mut result = VerificationResult {
        status: VerificationStatus::AnalysisIncomplete,
        validation,
        translation_errors: Vec::new(),
        translation_warnings: crate::validate::check_translation(&net),
        places: net.num_places(),
        transitions: net.num_transitions(),
        places_by_kind,
        input_arcs,
        output_arcs,
        cvn_dot: Some(unipn::export::to_dot(&net)),
        state_count: 0,
        analysis_complete: false,
        max_states: config.max_states,
        analysis_error: None,
        bugs: Vec::new(),
        unmet_goals: Vec::new(),
        goal_warnings: Vec::new(),
        declared_goal_count: program.goals.len(),
        timings: VerificationTimings {
            validation_ms,
            translation_ms,
            analysis_ms,
            ..VerificationTimings::default()
        },
    };

    result.state_count = analysis.state_count();
    if analysis.truncated {
        result.analysis_complete = false;
        result.analysis_error = Some(format!(
            "state space explosion: exceeded {} states",
            config.max_states
        ));
        result.timings.total_ms = elapsed_ms(total);
        return result;
    }
    result.analysis_complete = true;
    result.bugs = analyze(program, &net, &analysis);
    if !config.analyze_dead_transitions {
        result
            .bugs
            .retain(|bug| !matches!(bug.kind, crate::repair::BugKind::DeadTransition { .. }));
    }

    if config.check_goals && !program.goals.is_empty() {
        let goals_start = Instant::now();
        let (specs, mut warnings) = crate::translate_goals(program, &net);
        // A goal that already holds in the initial state constrains nothing
        // about the concurrent behavior: it would pass even if every thread
        // were deleted. Flag it as too weak instead of silently accepting.
        let initial = net.initial_state();
        for spec in &specs {
            if spec.satisfied_by(&initial) {
                warnings.push(format!(
                    "goal '{}' is already satisfied by the initial state and \
                     does not constrain any concurrent behavior (too weak)",
                    spec.id
                ));
            }
        }
        result.goal_warnings = warnings;
        result.unmet_goals = check_goals(&analysis, &specs);
        result.timings.goals_ms = elapsed_ms(goals_start);
    }

    result.status = if !result.bugs.is_empty() {
        VerificationStatus::VerifiedUnsafe
    } else if !result.unmet_goals.is_empty() || !result.goal_warnings.is_empty() {
        VerificationStatus::GoalsUnmet
    } else {
        VerificationStatus::VerifiedSafe
    };
    result.timings.total_ms = elapsed_ms(total);
    result
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
