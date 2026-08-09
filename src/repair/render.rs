use std::fmt::Write;

use super::report::{BugKind, BugReport, DeadlockParticipant};

const TEMPLATE_DEADLOCK: &str = include_str!("templates/deadlock.md");
const TEMPLATE_SIGNAL_LOSS: &str = include_str!("templates/signal_loss.md");
const TEMPLATE_CHANNEL_BLOCK: &str = include_str!("templates/channel_block.md");
const TEMPLATE_GOAL_UNMET: &str = include_str!("templates/goal_unmet.md");
const TEMPLATE_DEAD_TRANSITION: &str = include_str!("templates/dead_transition.md");

/// Render a bug report as human-readable text (also suitable as LLM input).
pub fn render_text(report: &BugReport) -> String {
    let mut out = String::new();

    write_header(&mut out, report);
    write_trace(&mut out, report);
    write_bug_details(&mut out, report);

    if let Some(hint) = &report.repair_hint {
        writeln!(out, "SUGGESTION: {hint}").unwrap();
    }

    out
}

/// Render an LLM repair prompt for a set of unmet business goals.
///
/// Used when the CVN analysis reports no concurrency bugs but one or
/// more `BusinessGoal`s are unreachable in the state space. The prompt
/// lists the unmet predicates, attaches the preservation constraints
/// derived from the current ConcIR, and appends the `goal_unmet.md`
/// strategy template.
pub fn render_goal_repair_prompt(
    program: &concir::ast::Program,
    unmet: &[cvn::analysis::UnmetGoal],
    original_cir_json: &str,
) -> String {
    let mut out = String::new();

    writeln!(out, "# Business Goal Repair Request\n").unwrap();
    writeln!(out, "## Status\n").unwrap();
    writeln!(
        out,
        "The ConcIR translates to a CVN with no concurrency bugs, but **{}** declared business goal(s) are unreachable.\n",
        unmet.len()
    )
    .unwrap();

    writeln!(out, "## Unmet Goals\n").unwrap();
    for g in unmet {
        let label = g
            .goal
            .desc
            .as_deref()
            .unwrap_or(g.goal.id.as_str());
        writeln!(out, "- `{}` ({})", g.goal.id, label).unwrap();
        writeln!(out, "  {}", g.reason).unwrap();
    }
    writeln!(out).unwrap();

    // Preservation constraints from the ConcIR (resources + protection + goals).
    let constraints = super::build_preservation_constraints(program);
    if !constraints.is_empty() {
        writeln!(out, "## Preservation Constraints\n").unwrap();
        for c in &constraints {
            writeln!(out, "- {c}").unwrap();
        }
        writeln!(out).unwrap();
    }

    writeln!(out, "{TEMPLATE_GOAL_UNMET}\n").unwrap();

    writeln!(out, "## Current ConcIR\n").unwrap();
    writeln!(out, "```json\n{original_cir_json}\n```\n").unwrap();

    writeln!(out, "## Output\n").unwrap();
    writeln!(
        out,
        "Output the complete revised ConcIR JSON. Do not drop any resource, protection entry, function, or goal."
    )
    .unwrap();

    out
}

/// Render a full LLM repair prompt following the paper's Table 4 structure:
/// Bug kind, Witness trace, Bug-state summary, Held resources,
/// Waiting relations, ConcIR slice, Preservation constraints,
/// Repair strategy (per-bug template), Current ConcIR, Output contract.
pub fn render_repair_prompt(report: &BugReport, original_cir_json: &str) -> String {
    let mut out = String::new();

    writeln!(out, "# Concurrency Bug Repair Request\n").unwrap();

    // 1. Bug kind
    writeln!(out, "## Bug Kind\n").unwrap();
    writeln!(out, "{}\n", report.kind.name()).unwrap();

    // 2. Witness trace (sid)
    write_witness_trace(&mut out, report);

    // 3 + 4 + 5. Bug-state summary, Held resources, Waiting relations
    write_state_summary(&mut out, report);

    // 6. Relevant ConcIR slice (Lambda)
    write_cir_slice(&mut out, report);

    // 7. Preservation constraints (Gamma_ctx)
    write_preservation(&mut out, report);

    // 8. Repair strategy (per-bug-type template with examples)
    write_repair_template(&mut out, report);

    // 9. Current ConcIR (full JSON)
    writeln!(out, "## Current ConcIR\n").unwrap();
    writeln!(out, "```json\n{original_cir_json}\n```\n").unwrap();

    // Output contract
    writeln!(out, "## Output\n").unwrap();
    writeln!(
        out,
        "Output the complete revised ConcIR JSON. Do not omit any function or resource."
    )
    .unwrap();

    out
}

// ── Section renderers ────────────────────────────────────────────

fn write_header(out: &mut String, report: &BugReport) {
    writeln!(out, "BUG: {}\n", report.summary).unwrap();
}

fn write_witness_trace(out: &mut String, report: &BugReport) {
    if report.trace.is_empty() {
        return;
    }
    writeln!(out, "## Witness Trace\n").unwrap();
    let sids: Vec<String> = report
        .trace
        .iter()
        .map(|step| {
            if step.anchor_sids.is_empty() {
                step.transition_id.clone()
            } else {
                step.anchor_sids.join(", ")
            }
        })
        .collect();
    writeln!(out, "{}\n", sids.join(" -> ")).unwrap();

    writeln!(out, "Detailed steps:\n").unwrap();
    for (i, step) in report.trace.iter().enumerate() {
        writeln!(out, "  {}. {}", i + 1, step.description).unwrap();
    }
    writeln!(out).unwrap();
}

fn write_state_summary(out: &mut String, report: &BugReport) {
    writeln!(out, "## Bug-State Summary\n").unwrap();

    match &report.kind {
        BugKind::Deadlock { participants } => {
            for p in participants {
                write_participant_state(out, p);
            }
        }
        BugKind::SignalLoss {
            notifier_tid,
            waiter_tid,
        } => {
            writeln!(out, "- Notifier ({notifier_tid}) executed notify before waiter entered wait").unwrap();
            writeln!(out, "- Waiter blocked at: {waiter_tid}").unwrap();
            writeln!(out, "- The notification was lost because waiter_count = 0 at notification time").unwrap();
        }
        BugKind::ChannelBlock {
            blocked_op,
            channel,
        } => {
            writeln!(out, "- Channel `{channel}`: `{blocked_op}` operation is permanently blocked").unwrap();
            writeln!(out, "- No matching counterpart can execute because of lock contention or missing pair").unwrap();
        }
        BugKind::DeadTransition { transition, sids } => {
            let sid_label = if sids.is_empty() {
                "(no anchor)".to_string()
            } else {
                sids.join(", ")
            };
            writeln!(
                out,
                "- CVN transition `{transition}` (sid: {sid_label}) is never enabled on any reachable path"
            )
            .unwrap();
            writeln!(
                out,
                "- The anchored ConcIR statement cannot execute regardless of interleaving"
            )
            .unwrap();
        }
    }

    if !report.final_marking_summary.is_empty() {
        writeln!(out, "\nFinal state: {}\n", report.final_marking_summary).unwrap();
    }

    // Involved resources and functions
    if !report.involved_resources.is_empty() {
        writeln!(
            out,
            "Involved resources: {}\n",
            report.involved_resources.join(", ")
        )
        .unwrap();
    }
    if !report.involved_functions.is_empty() {
        writeln!(
            out,
            "Involved functions: {}\n",
            report.involved_functions.join(", ")
        )
        .unwrap();
    }
}

fn write_participant_state(out: &mut String, p: &DeadlockParticipant) {
    let holding = if p.holding.is_empty() {
        "(none)".to_string()
    } else {
        format!("[{}]", p.holding.join(", "))
    };
    writeln!(
        out,
        "- `{}` at {}: holding {holding}, waiting for `{}`",
        p.function, p.blocked_at_sid, p.waiting_for
    )
    .unwrap();
}

fn write_cir_slice(out: &mut String, report: &BugReport) {
    if report.cir_slice.is_empty() {
        return;
    }
    writeln!(out, "## Relevant ConcIR Slice\n").unwrap();
    for entry in &report.cir_slice {
        writeln!(out, "- {}.{}: {}", entry.function, entry.sid, entry.op).unwrap();
    }
    writeln!(out).unwrap();
}

fn write_preservation(out: &mut String, report: &BugReport) {
    if report.preservation_constraints.is_empty() {
        return;
    }
    writeln!(out, "## Preservation Constraints\n").unwrap();
    for c in &report.preservation_constraints {
        writeln!(out, "- {c}").unwrap();
    }
    writeln!(out).unwrap();
}

fn write_repair_template(out: &mut String, report: &BugReport) {
    let template = match &report.kind {
        BugKind::Deadlock { .. } => TEMPLATE_DEADLOCK,
        BugKind::SignalLoss { .. } => TEMPLATE_SIGNAL_LOSS,
        BugKind::ChannelBlock { .. } => TEMPLATE_CHANNEL_BLOCK,
        BugKind::DeadTransition { .. } => TEMPLATE_DEAD_TRANSITION,
    };

    writeln!(out, "{template}\n").unwrap();

    // Append the instance-specific repair hint if available
    if let Some(hint) = &report.repair_hint {
        writeln!(out, "### Instance-Specific Hint\n").unwrap();
        writeln!(out, "{hint}\n").unwrap();
    }
}

fn write_trace(out: &mut String, report: &BugReport) {
    if report.trace.is_empty() {
        return;
    }
    writeln!(out, "TRACE ({} steps):", report.trace.len()).unwrap();
    for (i, step) in report.trace.iter().enumerate() {
        let sids = if step.anchor_sids.is_empty() {
            step.transition_id.clone()
        } else {
            step.anchor_sids.join(", ")
        };
        writeln!(out, "  {}. [{}] {}", i + 1, sids, step.description).unwrap();
    }
    writeln!(out).unwrap();
}

fn write_bug_details(out: &mut String, report: &BugReport) {
    match &report.kind {
        BugKind::Deadlock { participants } => {
            writeln!(out, "DEADLOCK:").unwrap();
            for p in participants {
                write_participant(out, p);
            }
        }
        BugKind::SignalLoss {
            notifier_tid,
            waiter_tid,
        } => {
            writeln!(out, "SIGNAL LOSS:").unwrap();
            writeln!(out, "  notifier: {notifier_tid}").unwrap();
            writeln!(out, "  waiter blocked at: {waiter_tid}").unwrap();
        }
        BugKind::ChannelBlock {
            blocked_op,
            channel,
        } => {
            writeln!(out, "CHANNEL BLOCK:").unwrap();
            writeln!(out, "  channel: {channel}, blocked on: {blocked_op}").unwrap();
        }
        BugKind::DeadTransition { transition, sids } => {
            writeln!(out, "DEAD TRANSITION:").unwrap();
            writeln!(out, "  transition: {transition}").unwrap();
            if !sids.is_empty() {
                writeln!(out, "  anchored sid(s): {}", sids.join(", ")).unwrap();
            }
        }
    }
    writeln!(out).unwrap();
    if !report.final_marking_summary.is_empty() {
        writeln!(out, "FINAL STATE: {}", report.final_marking_summary).unwrap();
        writeln!(out).unwrap();
    }
}

fn write_participant(out: &mut String, p: &DeadlockParticipant) {
    let holding = if p.holding.is_empty() {
        "(none)".to_string()
    } else {
        format!("[{}]", p.holding.join(", "))
    };
    writeln!(
        out,
        "  {}: holding {holding}, waiting for {} (blocked at {})",
        p.function, p.waiting_for, p.blocked_at_sid
    )
    .unwrap();
}
