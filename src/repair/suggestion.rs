use super::report::BugKind;

/// Generate an instance-specific repair hint from the detected bug's concrete data.
///
/// This produces a short, actionable hint that references the specific resources
/// and functions involved. The general repair strategy and CIR examples are
/// provided by the per-bug-type templates in `templates/*.md`.
pub fn suggestion_for(kind: &BugKind) -> Option<String> {
    match kind {
        BugKind::Deadlock { participants } => {
            if participants.is_empty() {
                return None;
            }
            let mut resources: Vec<&str> = participants
                .iter()
                .flat_map(|p| {
                    p.holding
                        .iter()
                        .map(String::as_str)
                        .chain(std::iter::once(p.waiting_for.as_str()))
                })
                .filter(|s| !s.is_empty())
                .collect();
            resources.sort();
            resources.dedup();
            let ordered = resources.join(" -> ");

            let changes: Vec<String> = participants
                .iter()
                .map(|p| {
                    format!(
                        "In function `{}`: reorder lock acquisition of `{}`",
                        p.function, p.waiting_for
                    )
                })
                .collect();

            Some(format!(
                "Enforce uniform lock ordering: {ordered}\n{}",
                changes.join("\n")
            ))
        }
        BugKind::SignalLoss {
            notifier_tid,
            waiter_tid,
        } => Some(format!(
            "Notifier ({notifier_tid}) may execute notify before waiter ({waiter_tid}) enters wait. \
             Add a while-loop checking the predicate variable before wait."
        )),
        BugKind::ChannelBlock {
            blocked_op,
            channel,
        } => Some(format!(
            "Channel `{channel}` {blocked_op} is blocked. \
             Move the {blocked_op} operation outside any held mutex."
        )),
    }
}
