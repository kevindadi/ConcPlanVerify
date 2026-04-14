//! Bug report generation and LLM repair infrastructure.
//!
//! This module converts low-level CVN counterexamples into enriched
//! [`BugReport`]s with CIR-level semantics, and can render them as
//! human-readable text or LLM repair prompts.

pub mod render;
pub mod report;
pub mod suggestion;

#[cfg(feature = "llm")]
pub mod llm;

pub use report::{BugKind, BugReport, DeadlockParticipant, EnrichedFiringStep};

use cvn::analysis::{AnalysisResult, Counterexample};
use cvn::model::{PlaceId, PlaceKind, TransitionId};
use cvn::net::CvnNet;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// Analyze CVN counterexamples and produce enriched bug reports.
///
/// Each CVN `Counterexample` (currently always `PropertyViolation::Deadlock`)
/// is classified into a more specific `BugKind` by inspecting the net
/// structure and final state.
pub fn analyze(
    program: &cir::ast::Program,
    net: &CvnNet,
    result: &AnalysisResult,
) -> Vec<BugReport> {
    let preservation = build_preservation_constraints(program);

    result
        .deadlocks
        .iter()
        .map(|cx| {
            let mut report = classify_counterexample(net, cx);
            report.cir_slice = extract_cir_slice(program, &report.trace);
            report.preservation_constraints = preservation.clone();
            report
        })
        .collect()
}

fn classify_counterexample(net: &CvnNet, cx: &Counterexample) -> BugReport {
    let blocked = cvn::analysis::blocked_places(net, &cx.final_state);

    let has_wait_place = blocked
        .iter()
        .any(|pid| net.place(pid).map(|p| p.is_wait()).unwrap_or(false));

    let has_signal_loss_trace = detect_signal_loss_in_trace(net, cx);

    let (kind, summary) = if has_wait_place || has_signal_loss_trace {
        classify_signal_loss(net, cx, &blocked)
    } else if let Some(channel_block) = classify_channel_block(net, &blocked) {
        channel_block
    } else {
        classify_deadlock(net, cx, &blocked)
    };

    let trace = enrich_trace(net, cx);
    let involved_resources = extract_involved_resources(net, &blocked);
    let involved_functions = extract_involved_functions(net, &blocked);
    let final_marking_summary = format_marking(net, &cx.final_state.marking);
    let repair_hint = suggestion::suggestion_for(&kind);

    BugReport {
        kind,
        trace,
        final_marking_summary,
        summary,
        involved_resources,
        involved_functions,
        cir_slice: Vec::new(),
        preservation_constraints: Vec::new(),
        repair_hint,
    }
}

/// Detect if a CondvarNotifyLost or CondvarNotifyAllLost transition
/// fired in the counterexample trace, indicating that a signal was
/// sent when no waiter was present.
fn detect_signal_loss_in_trace(net: &CvnNet, cx: &Counterexample) -> bool {
    cx.trace.iter().any(|step| {
        net.transition(&step.transition_id)
            .map(|t| {
                matches!(
                    t.kind,
                    cvn::model::TransitionKind::CondvarNotifyLost
                        | cvn::model::TransitionKind::CondvarNotifyAllLost
                )
            })
            .unwrap_or(false)
    })
}

fn classify_signal_loss(
    net: &CvnNet,
    cx: &Counterexample,
    blocked: &[PlaceId],
) -> (BugKind, String) {
    let mut waiter_tid = String::new();
    let mut notifier_tid = String::new();

    for pid in blocked {
        if let Some(place) = net.place(pid) {
            if let PlaceKind::Wait {
                cv_name,
                fn_name,
                sid,
            } = &place.kind
            {
                waiter_tid = format!("{fn_name}.{sid}");
                notifier_tid = format!("notify({cv_name})");
            }
        }
    }

    if waiter_tid.is_empty() {
        // Look for a CondvarNotifyLost/CondvarNotifyAllLost transition in the trace.
        for step in &cx.trace {
            if let Some(t) = net.transition(&step.transition_id) {
                if matches!(
                    t.kind,
                    cvn::model::TransitionKind::CondvarNotifyLost
                        | cvn::model::TransitionKind::CondvarNotifyAllLost
                ) {
                    notifier_tid = t.id.0.clone();
                    break;
                }
            }
        }
    }

    let summary = format!(
        "Signal loss detected: waiter blocked at {waiter_tid}, notify may have fired before wait"
    );
    let kind = BugKind::SignalLoss {
        notifier_tid,
        waiter_tid,
    };
    (kind, summary)
}

/// Check if a deadlock is actually a channel block: a blocked transition
/// requires tokens from a channel resource place.
fn classify_channel_block(net: &CvnNet, blocked: &[PlaceId]) -> Option<(BugKind, String)> {
    let place_consumers = build_place_to_consumers(net);

    for pid in blocked {
        let Some(consumers) = place_consumers.get(pid) else {
            continue;
        };

        for tid in consumers {
            for input_arc in net.input_arcs(tid) {
                if let Some(place) = net.place(&input_arc.place) {
                    if let PlaceKind::Resource {
                        res_name,
                        resource_type: cvn::model::ResourceType::Channel,
                    } = &place.kind
                    {
                        let kind_label = net
                            .transition(tid)
                            .map(|t| match t.kind {
                                cvn::model::TransitionKind::Send => "send",
                                cvn::model::TransitionKind::Recv => "recv",
                                _ => "recv",
                            })
                            .unwrap_or("recv");

                        let summary = format!(
                            "Channel block: {kind_label} on channel {res_name} has no matching counterpart"
                        );
                        return Some((
                            BugKind::ChannelBlock {
                                blocked_op: kind_label.to_string(),
                                channel: res_name.clone(),
                            },
                            summary,
                        ));
                    }
                }
            }
        }
    }

    None
}

fn classify_deadlock(net: &CvnNet, cx: &Counterexample, blocked: &[PlaceId]) -> (BugKind, String) {
    let participants = analyze_deadlock_participants(net, cx, blocked);

    let summary = if participants.is_empty() {
        "Deadlock detected".to_string()
    } else {
        let names: Vec<&str> = participants.iter().map(|p| p.function.as_str()).collect();
        format!("Deadlock detected involving {}", names.join(", "))
    };

    (BugKind::Deadlock { participants }, summary)
}

fn analyze_deadlock_participants(
    net: &CvnNet,
    cx: &Counterexample,
    blocked: &[PlaceId],
) -> Vec<DeadlockParticipant> {
    let place_consumers = build_place_to_consumers(net);
    let mut participants = Vec::new();

    for pid in blocked {
        let Some(place) = net.place(pid) else {
            continue;
        };

        let (fn_name, sid) = match &place.kind {
            PlaceKind::Control { fn_name, sid } => (fn_name.clone(), sid.clone()),
            PlaceKind::Wait { fn_name, sid, .. } => (fn_name.clone(), sid.clone()),
            _ => continue,
        };

        let waiting_for = find_waiting_resource(net, pid, &place_consumers);
        let holding = find_held_resources(net, &cx.final_state.marking, &fn_name, &place_consumers);

        participants.push(DeadlockParticipant {
            function: fn_name.clone(),
            blocked_at_sid: format!("{fn_name}.{sid}"),
            holding,
            waiting_for,
        });
    }

    participants
}

/// Build a map from place IDs to the transitions that consume tokens from them.
fn build_place_to_consumers(net: &CvnNet) -> HashMap<PlaceId, Vec<TransitionId>> {
    let mut map: HashMap<PlaceId, Vec<TransitionId>> = HashMap::new();
    for tid in net.transition_ids() {
        for arc in net.input_arcs(tid) {
            map.entry(arc.place.clone()).or_default().push(tid.clone());
        }
    }
    map
}

/// Find which resource a blocked place is waiting for by checking
/// outgoing transitions' input arcs for resource places without tokens.
fn find_waiting_resource(
    net: &CvnNet,
    blocked_place_id: &PlaceId,
    place_consumers: &HashMap<PlaceId, Vec<TransitionId>>,
) -> String {
    let consumers = match place_consumers.get(blocked_place_id) {
        Some(c) => c,
        None => return String::new(),
    };

    for tid in consumers {
        for input_arc in net.input_arcs(tid) {
            if let Some(place) = net.place(&input_arc.place) {
                if let PlaceKind::Resource { res_name, .. } = &place.kind {
                    return res_name.clone();
                }
            }
        }
    }
    String::new()
}

/// Find resources currently held by a function by looking at resource places
/// that have fewer tokens than their initial count, and whose consuming
/// transition belongs to this function.
fn find_held_resources(
    net: &CvnNet,
    marking: &cvn::model::Marking,
    fn_name: &str,
    place_consumers: &HashMap<PlaceId, Vec<TransitionId>>,
) -> Vec<String> {
    let mut held = Vec::new();
    let initial = net.initial_marking();

    for pid in net.place_ids() {
        let Some(place) = net.place(pid) else {
            continue;
        };
        let PlaceKind::Resource { res_name, .. } = &place.kind else {
            continue;
        };

        let init_tokens = initial.get(pid).copied().unwrap_or(0);
        let curr_tokens = marking.get(pid).copied().unwrap_or(0);

        if curr_tokens < init_tokens {
            if resource_consumed_by_function(pid, fn_name, place_consumers) {
                held.push(res_name.clone());
            }
        }
    }
    held
}

/// Check if any transition that consumes from this resource place
/// has an ID containing the function name (convention: "t_{fn_name}_...").
fn resource_consumed_by_function(
    resource_pid: &PlaceId,
    fn_name: &str,
    place_consumers: &HashMap<PlaceId, Vec<TransitionId>>,
) -> bool {
    let Some(consumers) = place_consumers.get(resource_pid) else {
        return false;
    };
    consumers.iter().any(|tid| tid.0.contains(fn_name))
}

fn enrich_trace(net: &CvnNet, cx: &Counterexample) -> Vec<EnrichedFiringStep> {
    cx.trace
        .iter()
        .map(|step| {
            let transition = net.transition(&step.transition_id);
            let kind = transition
                .map(|t| t.kind.clone())
                .unwrap_or(cvn::model::TransitionKind::Sequential);

            let anchor_sids: Vec<String> = step.anchor_sids.iter().cloned().collect();

            let description = format_step_description(&step.transition_id, &kind, &anchor_sids);

            EnrichedFiringStep {
                transition_id: step.transition_id.0.clone(),
                kind,
                anchor_sids,
                description,
            }
        })
        .collect()
}

fn format_step_description(
    tid: &TransitionId,
    kind: &cvn::model::TransitionKind,
    anchor_sids: &[String],
) -> String {
    use cvn::model::TransitionKind as TK;

    let kind_str = match kind {
        TK::Sequential => "sequential",
        TK::Lock => "lock",
        TK::Unlock => "unlock",
        TK::ReadLock => "read_lock",
        TK::ReadUnlock => "read_unlock",
        TK::Acquire => "acquire",
        TK::Release => "release",
        TK::Send => "send",
        TK::Recv => "recv",
        TK::VarRead => "var_read",
        TK::VarWrite => "var_write",
        TK::AtomicLoad => "atomic_load",
        TK::AtomicStore => "atomic_store",
        TK::BranchTrue => "branch_true",
        TK::BranchFalse => "branch_false",
        TK::Switch { label } => return format!("switch({label})"),
        TK::CasSuccess => "cas_success",
        TK::CasFailure => "cas_failure",
        TK::Spawn => "spawn",
        TK::Join => "join",
        TK::Call => "call",
        TK::CondvarWaitEnter => "condvar_wait_enter",
        TK::CondvarWakeByNotify => "condvar_wake_by_notify",
        TK::CondvarWakeByNotifyAll => "condvar_wake_by_notify_all",
        TK::CondvarReacquire => "condvar_reacquire",
        TK::CondvarNotify => "condvar_notify",
        TK::CondvarNotifyLost => "condvar_notify_lost",
        TK::CondvarNotifyAll => "condvar_notify_all",
        TK::CondvarNotifyAllLost => "condvar_notify_all_lost",
        TK::Return => "return",
        _ => "unknown",
    };

    if anchor_sids.is_empty() {
        format!("{kind_str} ({})", tid.0)
    } else {
        format!("{kind_str} — {}", anchor_sids.join(", "))
    }
}

fn extract_involved_resources(net: &CvnNet, blocked: &[PlaceId]) -> Vec<String> {
    let place_consumers = build_place_to_consumers(net);
    let mut resources = HashSet::new();

    for pid in blocked {
        let Some(consumers) = place_consumers.get(pid) else {
            continue;
        };
        for tid in consumers {
            for input_arc in net.input_arcs(tid) {
                if let Some(place) = net.place(&input_arc.place) {
                    if let PlaceKind::Resource { res_name, .. } = &place.kind {
                        resources.insert(res_name.clone());
                    }
                }
            }
        }
    }

    let mut sorted: Vec<String> = resources.into_iter().collect();
    sorted.sort();
    sorted
}

fn extract_involved_functions(net: &CvnNet, blocked: &[PlaceId]) -> Vec<String> {
    let mut functions = HashSet::new();

    for pid in blocked {
        if let Some(place) = net.place(pid) {
            match &place.kind {
                PlaceKind::Control { fn_name, .. } | PlaceKind::Wait { fn_name, .. } => {
                    functions.insert(fn_name.clone());
                }
                _ => {}
            }
        }
    }

    let mut sorted: Vec<String> = functions.into_iter().collect();
    sorted.sort();
    sorted
}

fn format_marking(net: &CvnNet, marking: &cvn::model::Marking) -> String {
    let mut parts = Vec::new();
    let mut entries: Vec<_> = marking.iter().filter(|(_, count)| **count > 0).collect();
    entries.sort_by_key(|(pid, _)| &pid.0);

    for (pid, count) in entries {
        let label = if let Some(place) = net.place(pid) {
            match &place.kind {
                PlaceKind::Control { fn_name, sid } => format!("{fn_name}.{sid}"),
                PlaceKind::Resource { res_name, .. } => format!("R({res_name})"),
                PlaceKind::Wait {
                    cv_name,
                    fn_name,
                    sid,
                } => {
                    format!("W({cv_name}@{fn_name}.{sid})")
                }
            }
        } else {
            pid.0.clone()
        };
        if *count == 1 {
            parts.push(label);
        } else {
            parts.push(format!("{label}×{count}"));
        }
    }

    let mut out = String::new();
    write!(out, "{{{}}}", parts.join(", ")).unwrap();
    out
}

/// Extract CIR statements relevant to the bug trace (Lambda in the diagnostic tuple).
fn extract_cir_slice(
    program: &cir::ast::Program,
    trace: &[report::EnrichedFiringStep],
) -> Vec<report::CirSliceEntry> {
    let trace_sids: HashSet<String> = trace
        .iter()
        .flat_map(|step| step.anchor_sids.iter().cloned())
        .collect();

    let mut entries = Vec::new();
    for func in &program.functions {
        for stmt in &func.body {
            if trace_sids.contains(&stmt.sid) {
                entries.push(report::CirSliceEntry {
                    sid: stmt.sid.clone(),
                    op: format!("{:?}", stmt.op),
                    function: func.name.clone(),
                });
            }
        }
    }
    entries
}

/// Build preservation constraints from the CIR program (Gamma_ctx).
fn build_preservation_constraints(program: &cir::ast::Program) -> Vec<String> {
    let mut constraints = Vec::new();

    for res in &program.resources {
        constraints.push(format!(
            "Resource '{}' (kind={}, type={}) must remain in the artifact",
            res.name, res.kind, res.res_type
        ));
    }

    for prot in &program.protection {
        constraints.push(format!(
            "Variable '{}' must remain protected by '{}'",
            prot.var, prot.lock
        ));
    }

    for goal in &program.goals {
        let desc = goal.desc.as_deref().unwrap_or(&goal.id);
        constraints.push(format!("Business goal '{}' must remain achievable", desc));
    }

    constraints
}
