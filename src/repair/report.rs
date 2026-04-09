use cvn::model::TransitionKind;
use serde::{Deserialize, Serialize};

/// Classification of a detected concurrency bug.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BugKind {
    /// No transitions enabled and not all threads have returned.
    Deadlock {
        participants: Vec<DeadlockParticipant>,
    },
    /// A condvar notify fires before the waiter reaches its wait point,
    /// causing the signal to be lost and the waiter to block forever.
    SignalLoss {
        /// Transition ID that performed the notify.
        notifier_tid: String,
        /// Transition ID (or wait-place) where the waiter is stuck.
        waiter_tid: String,
    },
    /// A channel send/recv is blocked with no matching counterpart.
    ChannelBlock {
        /// "send" or "recv"
        blocked_op: String,
        /// Channel resource name.
        channel: String,
    },
}

impl BugKind {
    /// Short name for display/assertion purposes.
    pub fn name(&self) -> &'static str {
        match self {
            BugKind::Deadlock { .. } => "Deadlock",
            BugKind::SignalLoss { .. } => "SignalLoss",
            BugKind::ChannelBlock { .. } => "ChannelBlock",
        }
    }
}

/// A thread involved in a deadlock.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeadlockParticipant {
    /// CIR function name.
    pub function: String,
    /// Statement ID where this thread is blocked (e.g. "w1.s2").
    pub blocked_at_sid: String,
    /// Resource names currently held by this thread.
    pub holding: Vec<String>,
    /// Resource name this thread is waiting for.
    pub waiting_for: String,
}

/// A single step in the counterexample trace, enriched with CIR-level info.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnrichedFiringStep {
    /// CVN transition ID (e.g. "t_w1_s1_lock").
    pub transition_id: String,
    /// Classification of the transition.
    pub kind: TransitionKind,
    /// CIR statement IDs anchored to this transition.
    pub anchor_sids: Vec<String>,
    /// Human-readable description (e.g. "[w1.s1] lock(mtx_a)").
    pub description: String,
}

/// A complete bug report combining CVN analysis with CIR-level semantics.
///
/// Corresponds to the diagnostic tuple D = (kappa, pi_mu, Sigma_state,
/// Sigma_wait, Lambda, Gamma_ctx, H) from the paper.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BugReport {
    /// Classification of the bug (kappa).
    pub kind: BugKind,
    /// Firing sequence leading to the bug (pi_mu).
    pub trace: Vec<EnrichedFiringStep>,
    /// Human-readable summary of the final state marking (Sigma_state).
    pub final_marking_summary: String,
    /// One-line summary of the bug.
    pub summary: String,
    /// Resource names involved in the bug.
    pub involved_resources: Vec<String>,
    /// CIR function names involved in the bug.
    pub involved_functions: Vec<String>,
    /// CIR statements relevant to the bug (Lambda).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cir_slice: Vec<CirSliceEntry>,
    /// Preservation constraints: resource/protection/goal invariants (Gamma_ctx).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preservation_constraints: Vec<String>,
    /// Heuristic repair hint (H).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_hint: Option<String>,
}

/// A CIR statement entry in the bug report's CIR slice (Lambda).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CirSliceEntry {
    pub sid: String,
    pub op: String,
    pub function: String,
}
