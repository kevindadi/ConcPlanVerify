//! Goal reachability analysis for CVN.
//!
//! A *business goal* is a user-defined predicate over reachable CVN states
//! that encodes the functional outcome a concurrent program is supposed to
//! achieve. Unlike deadlock detection (which asks "is every reachable
//! terminal state a proper termination?"), goal reachability asks
//! "does some reachable state witness the intended outcome?".
//!
//! Each [`GoalSpec`] is evaluated as an **EF (exists finally)** query:
//! the goal is satisfied iff there is a reachable state `s` in which every
//! [`GoalPredicate`] inside the spec holds simultaneously.
//!
//! The analysis is used by the repair loop as a *third verification layer*:
//! even when [`super::search::explore`] reports no deadlocks, an unmet goal
//! indicates that a repair dropped functional behavior and should trigger
//! another repair round.

use crate::analysis::search::{AnalysisConfig, explore};
use crate::error::CvnError;
use crate::model::{ConcreteVal, PlaceId, State, Val};
use crate::net::CvnNet;
use serde::{Deserialize, Serialize};

/// A primitive predicate over a single CVN state.
///
/// Predicates are combined through [`GoalSpec::predicates`] using a
/// conjunction: a state satisfies a spec iff every predicate holds in
/// that state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalPredicate {
    /// The place holds at least `min_tokens` tokens (marking threshold).
    ///
    /// Useful for "thread X reached its return place" or
    /// "the channel has at least one pending message".
    Reachable {
        /// Place whose token count is examined.
        place: PlaceId,
        /// Minimum number of tokens required to satisfy the predicate.
        min_tokens: u32,
    },

    /// The place holds exactly zero tokens.
    ///
    /// Useful for "no residual condvar signal" or "no pending channel
    /// messages at the end of the run". Semantically stronger than
    /// `Reachable { min_tokens: 0 }` (which is trivially true).
    Empty {
        /// Place whose token count must be zero.
        place: PlaceId,
    },

    /// The global variable equals the given concrete value.
    ///
    /// Evaluated against the CVN variable store. `Val::Unknown` never
    /// matches a concrete value, which keeps the analysis sound under
    /// three-valued guards (over-approximated branches cannot spuriously
    /// satisfy a functional goal).
    GlobalEq {
        /// Name of the variable in the CVN variable store.
        var: String,
        /// Expected concrete value.
        value: ConcreteVal,
    },
}

impl GoalPredicate {
    /// Evaluate the predicate against a state.
    pub fn holds(&self, state: &State) -> bool {
        match self {
            Self::Reachable { place, min_tokens } => state.tokens(place) >= *min_tokens,
            Self::Empty { place } => state.tokens(place) == 0,
            Self::GlobalEq { var, value } => match state.vars.get(var) {
                Some(Val::Concrete(c)) => c == value,
                _ => false,
            },
        }
    }
}

/// A named business goal: a conjunction of predicates that must be
/// simultaneously satisfied in some reachable state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalSpec {
    /// Stable identifier (mirrors the originating ConcIR goal id).
    pub id: String,

    /// Optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,

    /// All predicates must hold in the same reachable state.
    pub predicates: Vec<GoalPredicate>,
}

impl GoalSpec {
    /// Check whether `state` witnesses this spec.
    pub fn satisfied_by(&self, state: &State) -> bool {
        self.predicates.iter().all(|p| p.holds(state))
    }
}

/// A goal that was not witnessed by any reachable state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnmetGoal {
    /// The goal that failed to be satisfied.
    pub goal: GoalSpec,
    /// Short diagnostic explaining which predicate(s) were never met.
    pub reason: String,
}

/// Check whether each goal spec is reachable in `net`.
///
/// The analysis runs a single full state-space exploration and then
/// evaluates every goal against every reachable state. Returns the
/// specs that were never satisfied.
///
/// Errors propagate from [`explore`] (typically state-space explosion).
pub fn check_goals(
    net: &CvnNet,
    specs: &[GoalSpec],
    config: &AnalysisConfig,
) -> Result<Vec<UnmetGoal>, CvnError> {
    if specs.is_empty() {
        return Ok(Vec::new());
    }

    let result = explore(net, config)?;
    Ok(check_goals_in_result(&result, specs))
}

/// Check goals against an already completed state-space exploration.
///
/// Callers that also need deadlock analysis can use this function to avoid
/// exploring the same network a second time.
pub fn check_goals_in_result(
    result: &crate::analysis::search::AnalysisResult,
    specs: &[GoalSpec],
) -> Vec<UnmetGoal> {
    if specs.is_empty() {
        return Vec::new();
    }

    let mut satisfied = vec![false; specs.len()];

    for node_idx in result.reachability_graph.node_indices() {
        let state = &result.reachability_graph[node_idx];
        for (i, spec) in specs.iter().enumerate() {
            if !satisfied[i] && spec.satisfied_by(state) {
                satisfied[i] = true;
            }
        }
        if satisfied.iter().all(|b| *b) {
            break;
        }
    }

    let unmet = specs
        .iter()
        .zip(satisfied.iter())
        .filter(|&(_, &ok)| !ok)
        .map(|(spec, _)| UnmetGoal {
            reason: explain_unmet(spec),
            goal: spec.clone(),
        })
        .collect();

    unmet
}

fn explain_unmet(spec: &GoalSpec) -> String {
    let parts: Vec<String> = spec
        .predicates
        .iter()
        .map(|p| match p {
            GoalPredicate::Reachable { place, min_tokens } => {
                format!("M({}) >= {}", place, min_tokens)
            }
            GoalPredicate::Empty { place } => format!("M({}) == 0", place),
            GoalPredicate::GlobalEq { var, value } => format!("{} == {}", var, value),
        })
        .collect();

    format!(
        "No reachable state satisfies: {}",
        parts.join(" AND ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::CvnNetBuilder;
    use crate::model::{BoolExpr, TransitionKind};

    fn two_step_net() -> CvnNet {
        CvnNetBuilder::new()
            .add_control_place("p0", "main", "s0")
            .add_control_place("p1", "main", "s1")
            .set_return("p1")
            .add_transition("t0", TransitionKind::Sequential)
            .add_input_arc("p0", "t0", 1, BoolExpr::True)
            .add_output_arc("t0", "p1", 1, None)
            .set_initial_tokens("p0", 1)
            .build()
            .unwrap()
    }

    #[test]
    fn reaches_terminal_place() {
        let net = two_step_net();
        let specs = vec![GoalSpec {
            id: "g_done".into(),
            desc: None,
            predicates: vec![GoalPredicate::Reachable {
                place: PlaceId::new("p1"),
                min_tokens: 1,
            }],
        }];
        let unmet = check_goals(&net, &specs, &AnalysisConfig::default()).unwrap();
        assert!(unmet.is_empty(), "goal should be reachable, got {:?}", unmet);
    }

    #[test]
    fn unreachable_place_is_reported() {
        let net = two_step_net();
        let specs = vec![GoalSpec {
            id: "g_impossible".into(),
            desc: Some("token never produced here".into()),
            predicates: vec![GoalPredicate::Reachable {
                place: PlaceId::new("p_missing"),
                min_tokens: 1,
            }],
        }];
        let unmet = check_goals(&net, &specs, &AnalysisConfig::default()).unwrap();
        assert_eq!(unmet.len(), 1);
        assert_eq!(unmet[0].goal.id, "g_impossible");
    }

    #[test]
    fn empty_predicate_holds_initially() {
        let net = two_step_net();
        let specs = vec![GoalSpec {
            id: "g_empty".into(),
            desc: None,
            predicates: vec![GoalPredicate::Empty {
                place: PlaceId::new("p1"),
            }],
        }];
        let unmet = check_goals(&net, &specs, &AnalysisConfig::default()).unwrap();
        assert!(unmet.is_empty());
    }
}
