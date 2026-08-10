//! Business-goal types and reachability checking.
//!
//! ConcIR `BusinessGoal`s are translated (see `translator/goals.rs`) into
//! [`GoalSpec`]s over the built net, then checked against the reachability
//! graph: a goal is met iff some reachable state satisfies every predicate.

use serde::Serialize;
use unipn::analysis::ReachabilityGraph;
use unipn::{ConcreteVal, PlaceId, State, Val};

/// A single predicate over a state.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum GoalPredicate {
    /// The place holds at least `min_tokens` tokens.
    Reachable { place: PlaceId, min_tokens: u32 },
    /// The place holds no tokens (empty).
    Empty { place: PlaceId },
    /// A global variable equals the given value.
    GlobalEq { var: String, value: ConcreteVal },
}

impl GoalPredicate {
    pub fn satisfied_by(&self, state: &State) -> bool {
        match self {
            GoalPredicate::Reachable { place, min_tokens } => {
                state.marking.tokens(*place) >= *min_tokens
            }
            GoalPredicate::Empty { place } => state.marking.tokens(*place) == 0,
            GoalPredicate::GlobalEq { var, value } => {
                state.vars().get(var) == Some(&Val::Concrete(value.clone()))
            }
        }
    }
}

/// A translated business goal.
#[derive(Clone, Debug, Serialize)]
pub struct GoalSpec {
    pub id: String,
    pub desc: Option<String>,
    pub predicates: Vec<GoalPredicate>,
}

impl GoalSpec {
    pub fn satisfied_by(&self, state: &State) -> bool {
        self.predicates.iter().all(|p| p.satisfied_by(state))
    }
}

/// A goal no reachable state satisfies.
#[derive(Clone, Debug, Serialize)]
pub struct UnmetGoal {
    pub goal: GoalSpec,
    pub reason: String,
}

/// Check which goals are unreachable in the given reachability graph.
pub fn check_goals(rg: &ReachabilityGraph, specs: &[GoalSpec]) -> Vec<UnmetGoal> {
    specs
        .iter()
        .filter_map(|spec| {
            let met = rg.states.iter().any(|s| spec.satisfied_by(s));
            if met {
                None
            } else {
                Some(UnmetGoal {
                    goal: spec.clone(),
                    reason: "no reachable state satisfies all declared predicates".to_string(),
                })
            }
        })
        .collect()
}
