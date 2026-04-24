use cvn::model::TransitionKind;
use serde::{Deserialize, Serialize};

/// Classification of a detected concurrency bug.
///
/// # Classification hierarchy
///
/// Two bug classes are directly detected by the CVN state-space search:
/// [`BugKind::Deadlock`] — a state with no enabled transitions where at
/// least one thread has not reached its return place — and
/// [`BugKind::DeadTransition`] — a transition that never fires on any
/// edge of the reachability graph (its anchored CIR statement is
/// behaviorally unreachable). Both are primary, independent soundness
/// claims.
///
/// The variants [`BugKind::SignalLoss`] and [`BugKind::ChannelBlock`]
/// are *secondary sub-classifications*: the repair layer inspects the
/// counterexample trace and the set of blocked control/wait places of
/// an already-reported deadlock, and relabels it with the more specific
/// variant when the evidence is unambiguous. They never broaden the
/// set of reported bugs beyond the deadlocks found by
/// [`cvn::analysis::explore`], which keeps the analysis *sound* (no
/// false positives): any state labelled `SignalLoss` or `ChannelBlock`
/// is also a genuine deadlock in the CVN semantics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BugKind {
    /// No transitions enabled and not all threads have returned.
    ///
    /// Primary classification produced directly by
    /// [`cvn::analysis::explore`]. This is the only variant that is
    /// sound on its own — the sub-classifications below refine a
    /// `Deadlock` counterexample but do not add new ones.
    Deadlock {
        participants: Vec<DeadlockParticipant>,
    },
    /// **Sub-classification of a deadlock** in which the blocking
    /// context is a lost condvar notification.
    ///
    /// Emitted only when
    ///
    /// 1. an already-detected deadlock state has at least one token on a
    ///    [`cvn::model::PlaceKind::Wait`] place, *or*
    /// 2. the counterexample trace contains a `CondvarNotifyLost`
    ///    (`CondvarNotifyAllLost`) transition firing before the waiter
    ///    reached its wait point.
    ///
    /// This variant is therefore **not** an independent soundness claim
    /// about signal-loss detection in general — `notify` firings without
    /// a subsequent deadlock are intentionally ignored, because
    /// scheduler-agnostic reasoning on the CVN alone cannot guarantee
    /// zero false positives for standalone signal-loss detection.
    SignalLoss {
        /// Transition ID that performed the notify.
        notifier_tid: String,
        /// Transition ID (or wait-place) where the waiter is stuck.
        waiter_tid: String,
    },
    /// **Sub-classification of a deadlock** in which the blocked thread
    /// is waiting on a channel resource place.
    ///
    /// Like [`BugKind::SignalLoss`], this variant refines an existing
    /// deadlock counterexample; it never adds new reachable bugs.
    ChannelBlock {
        /// "send" or "recv"
        blocked_op: String,
        /// Channel resource name.
        channel: String,
    },
    /// **Primary classification**: a transition that never fires on any
    /// reachable edge of the CVN state graph.
    ///
    /// Corresponds to [`cvn::analysis::PropertyViolation::DeadTransition`].
    /// The anchored CIR statement is behaviorally unreachable; this
    /// typically indicates unreachable code (e.g., a `branch` whose
    /// condition is statically falsified) or a missing producer.
    DeadTransition {
        /// CVN transition identifier that never fires.
        transition: String,
        /// CIR statement id(s) anchored to the dead transition, if any.
        sids: Vec<String>,
    },
}

impl BugKind {
    /// Short name for display/assertion purposes.
    pub fn name(&self) -> &'static str {
        match self {
            BugKind::Deadlock { .. } => "Deadlock",
            BugKind::SignalLoss { .. } => "SignalLoss",
            BugKind::ChannelBlock { .. } => "ChannelBlock",
            BugKind::DeadTransition { .. } => "DeadTransition",
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
