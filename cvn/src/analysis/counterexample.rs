//! Counterexample and firing step types for reporting property violations.

use crate::model::{State, TransitionId};
use serde::{Deserialize, Serialize};
#[cfg(feature = "cir-anchor")]
use smallvec::SmallVec;

/// The kind of property violation detected by the CVN state-space search.
///
/// Only **deadlock** is a primary, stand-alone property violation that
/// `explore()` can report. Related bug classes such as *signal loss* and
/// *channel block* are downstream **secondary classifications** performed
/// by [`crate::`]-external tooling (see `src/repair/mod.rs` in the
/// `cir2cvn` crate): they inspect a deadlock counterexample's trace and
/// blocked places to produce a finer-grained diagnostic, but they never
/// add new reachable counterexamples beyond those already returned here.
///
/// The variant is kept `#[non_exhaustive]` so that future property
/// violations (e.g. fairness-backed liveness, temporal assertions) can be
/// added without a breaking change — but adding a variant must be
/// accompanied by a sound CVN-level detector in [`crate::analysis`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PropertyViolation {
    /// Deadlock: no transitions enabled and not all threads have returned.
    Deadlock,
}

/// A single step in a counterexample trace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiringStep {
    /// The transition that fired.
    pub transition_id: TransitionId,
    /// The CIR statement IDs anchored to this transition (μ(t)).
    /// Only available with the `cir-anchor` feature.
    #[cfg(feature = "cir-anchor")]
    #[serde(default, skip_serializing_if = "SmallVec::is_empty")]
    pub anchor_sids: SmallVec<[String; 2]>,
}

/// A counterexample demonstrating a property violation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Counterexample {
    /// The kind of violation.
    pub kind: PropertyViolation,
    /// The sequence of firing steps leading to the violation.
    pub trace: Vec<FiringStep>,
    /// The final (violating) state.
    pub final_state: State,
}
