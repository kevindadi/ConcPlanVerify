use cvn::net::CvnNet;

/// Optional post-translation sanity checks.
///
/// These are lightweight structural checks that complement the CVN builder's
/// own well-formedness validation. They catch translation bugs rather than
/// CIR input errors.
pub fn check_translation(net: &CvnNet) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check 1: Every non-resource, non-wait place should have at least one
    // incoming or outgoing arc (i.e. it participates in the net).
    for place in net.places() {
        if place.is_resource() || place.is_wait() {
            continue;
        }
        let pid = &place.id;
        let has_incoming = net
            .transitions()
            .any(|t| net.output_arcs(&t.id).iter().any(|a| a.place == *pid));
        let has_outgoing = net
            .transitions()
            .any(|t| net.input_arcs(&t.id).iter().any(|a| a.place == *pid));
        let has_initial_token = net.initial_marking().get(pid).copied().unwrap_or(0) > 0;

        if !has_incoming && !has_outgoing && !has_initial_token {
            warnings.push(format!("orphan control place: {pid}"));
        }
    }

    // Check 2: Every transition should have at least one input arc.
    for t in net.transitions() {
        if net.input_arcs(&t.id).is_empty() {
            warnings.push(format!("transition {} has no input arcs", t.id));
        }
    }

    // Check 3: Every transition should have at least one output arc.
    for t in net.transitions() {
        if net.output_arcs(&t.id).is_empty() {
            warnings.push(format!("transition {} has no output arcs", t.id));
        }
    }

    warnings
}
