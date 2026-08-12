use unipn::model::{ControlSub, PlaceKind};
use unipn::{CvnNet, CvnState, TransitionId};

/// Optional post-translation sanity checks.
///
/// These are lightweight structural checks that complement the net builder's
/// own well-formedness validation. They catch translation bugs rather than
/// ConcIR input errors.
pub fn check_translation(net: &CvnNet, initial: &CvnState) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check 1: Every non-resource, non-wait control place should have at least
    // one incoming or outgoing arc (i.e. it participates in the net).
    for place in &net.places {
        let is_wait = matches!(place.kind, PlaceKind::Control(ControlSub::WaitPoint));
        if is_wait || !net.is_control_flow(place.id) {
            continue;
        }
        let pid = place.id;
        let has_incoming = net
            .transitions
            .iter()
            .any(|t| net.post_arcs(t.id).iter().any(|arc| arc.place == pid));
        let has_outgoing = net
            .transitions
            .iter()
            .any(|t| net.pre_arcs(t.id).iter().any(|arc| arc.place == pid));
        let has_initial_token = initial.marking.tokens(pid) > 0;

        if !has_incoming && !has_outgoing && !has_initial_token {
            warnings.push(format!("orphan control place: {}", place.name));
        }
    }

    // Check 2: Every transition should have at least one input arc.
    for t in 0..net.num_transitions() {
        let tid = TransitionId(t);
        if net.pre_arcs(tid).is_empty() {
            warnings.push(format!(
                "transition {} has no input arcs",
                net.transition_label(tid)
            ));
        }
    }

    // Check 3: Every transition should have at least one output arc.
    for t in 0..net.num_transitions() {
        let tid = TransitionId(t);
        if net.post_arcs(tid).is_empty() {
            warnings.push(format!(
                "transition {} has no output arcs",
                net.transition_label(tid)
            ));
        }
    }

    warnings
}
