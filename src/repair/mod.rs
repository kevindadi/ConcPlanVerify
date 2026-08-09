//! Bug report generation and LLM repair infrastructure.
//!
//! This module converts low-level CVN counterexamples into enriched
//! [`BugReport`]s with ConcIR-level semantics, and can render them as
//! human-readable text or LLM repair prompts.

pub mod render;
pub mod report;
pub mod suggestion;

pub use report::{BugKind, BugReport, DeadlockParticipant, EnrichedFiringStep};

use cvn::analysis::{AnalysisResult, Counterexample, PropertyViolation};
use cvn::model::{PlaceId, PlaceKind, TransitionId};
use cvn::net::CvnNet;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// Analyze CVN counterexamples and produce enriched bug reports.
///
/// Each CVN [`Counterexample`] is classified into a more specific
/// [`BugKind`] by inspecting the net structure and the final state.
/// Deadlocks come from `result.deadlocks`; behavioral dead transitions
/// are computed from the reachability graph via
/// [`cvn::analysis::find_dead_transitions`].
pub fn analyze(
    program: &concir::ast::Program,
    net: &CvnNet,
    result: &AnalysisResult,
) -> Vec<BugReport> {
    let preservation = build_preservation_constraints(program);
    let fn_to_module: HashMap<String, String> = program
        .functions
        .iter()
        .filter_map(|f| f.module.as_ref().map(|m| (f.name.clone(), m.clone())))
        .collect();

    let mut reports: Vec<BugReport> = result
        .deadlocks
        .iter()
        .map(|cx| {
            let mut report = classify_counterexample(net, cx, &fn_to_module);
            report.involved_modules = modules_for_functions(&report.involved_functions, &fn_to_module);
            report.cir_slice = extract_cir_slice(program, &report.trace);
            report.preservation_constraints = preservation.clone();
            report
        })
        .collect();

    let deadlock_suffixes = deadlock_dominated_dead_transitions(net, result);
    for cx in cvn::analysis::find_dead_transitions(net, result) {
        let is_deadlock_suffix = match &cx.kind {
            PropertyViolation::DeadTransition { transition_id, .. } => {
                deadlock_suffixes.contains(transition_id)
            }
            _ => false,
        };
        if is_deadlock_suffix {
            continue;
        }

        let mut report = classify_counterexample(net, &cx, &fn_to_module);
        if let BugKind::DeadTransition { sids, .. } = &report.kind
            && report.involved_functions.is_empty()
        {
            report.involved_functions = functions_for_sids(program, sids);
        }
        report.involved_modules = modules_for_functions(&report.involved_functions, &fn_to_module);
        report.cir_slice = extract_cir_slice(program, &report.trace);
        report.preservation_constraints = preservation.clone();
        reports.push(report);
    }

    reports
}

/// Find dead transitions that are only unreachable because exploration stops at
/// a reachable deadlock. This keeps independent unreachable-code diagnostics
/// while avoiding downstream repair targets for the same deadlock.
fn deadlock_dominated_dead_transitions(
    net: &CvnNet,
    result: &AnalysisResult,
) -> HashSet<TransitionId> {
    let roots: HashSet<PlaceId> = result
        .deadlocks
        .iter()
        .flat_map(|cx| cvn::analysis::blocked_places(net, &cx.final_state))
        .filter(|pid| net.place(pid).map(|p| p.is_control_flow()).unwrap_or(false))
        .collect();

    if roots.is_empty() {
        return HashSet::new();
    }

    let mut successors: HashMap<PlaceId, HashSet<PlaceId>> = HashMap::new();
    for tid in net.transition_ids() {
        let inputs: Vec<PlaceId> = net
            .input_arcs(tid)
            .into_iter()
            .filter(|arc| {
                net.place(&arc.place)
                    .map(|place| place.is_control_flow())
                    .unwrap_or(false)
            })
            .map(|arc| arc.place.clone())
            .collect();
        let outputs: Vec<PlaceId> = net
            .output_arcs(tid)
            .into_iter()
            .filter(|arc| {
                net.place(&arc.place)
                    .map(|place| place.is_control_flow())
                    .unwrap_or(false)
            })
            .map(|arc| arc.place.clone())
            .collect();

        for input in &inputs {
            for output in &outputs {
                successors
                    .entry(input.clone())
                    .or_default()
                    .insert(output.clone());
            }
        }
    }

    let mut downstream = roots.clone();
    let mut pending: Vec<PlaceId> = roots.into_iter().collect();
    while let Some(place) = pending.pop() {
        if let Some(next_places) = successors.get(&place) {
            for next in next_places {
                if downstream.insert(next.clone()) {
                    pending.push(next.clone());
                }
            }
        }
    }

    cvn::analysis::find_dead_transitions(net, result)
        .into_iter()
        .filter_map(|cx| match cx.kind {
            PropertyViolation::DeadTransition { transition_id, .. }
                if net.input_arcs(&transition_id).into_iter().any(|arc| {
                    downstream.contains(&arc.place)
                        && net
                            .place(&arc.place)
                            .map(|place| place.is_control_flow())
                            .unwrap_or(false)
                }) => Some(transition_id),
            _ => None,
        })
        .collect()
}

fn functions_for_sids(program: &concir::ast::Program, sids: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for func in &program.functions {
        if func.body.iter().any(|stmt| sids.contains(&stmt.sid)) {
            if !result.contains(&func.name) {
                result.push(func.name.clone());
            }
        }
    }
    result.sort();
    result
}

/// Distinct source modules of the given functions, sorted.
fn modules_for_functions(
    functions: &[String],
    fn_to_module: &HashMap<String, String>,
) -> Vec<String> {
    let mut modules: Vec<String> = functions
        .iter()
        .filter_map(|f| fn_to_module.get(f).cloned())
        .collect();
    modules.sort();
    modules.dedup();
    modules
}

fn classify_counterexample(
    net: &CvnNet,
    cx: &Counterexample,
    fn_to_module: &HashMap<String, String>,
) -> BugReport {
    if let PropertyViolation::DeadTransition { .. } = &cx.kind {
        return classify_dead_transition(net, cx, fn_to_module);
    }
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
        classify_deadlock(net, cx, &blocked, fn_to_module)
    };

    let trace = enrich_trace(net, cx, fn_to_module);
    let involved_resources = extract_involved_resources(net, &blocked);
    let involved_functions = extract_involved_functions(net, &blocked);
    let involved_modules = modules_for_functions(&involved_functions, fn_to_module);
    let final_marking_summary = format_marking(net, &cx.final_state.marking);
    let repair_hint = suggestion::suggestion_for(&kind);

    BugReport {
        kind,
        trace,
        final_marking_summary,
        summary,
        involved_resources,
        involved_functions,
        involved_modules,
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

fn classify_dead_transition(
    net: &CvnNet,
    cx: &Counterexample,
    fn_to_module: &HashMap<String, String>,
) -> BugReport {
    let (transition_id_str, sids): (String, Vec<String>) = match &cx.kind {
        PropertyViolation::DeadTransition {
            transition_id,
            anchor_sids,
        } => (
            transition_id.0.clone(),
            anchor_sids.iter().cloned().collect(),
        ),
        _ => (String::new(), Vec::new()),
    };

    let source_function = net
        .transition(&TransitionId::new(transition_id_str.clone()))
        .and_then(|t| t.source_function.clone());
    let involved_functions: Vec<String> = source_function.iter().cloned().collect();

    let anchor_label = if sids.is_empty() {
        format!("transition {}", transition_id_str)
    } else {
        format!(
            "transition {} (sid: {})",
            transition_id_str,
            sids.join(", ")
        )
    };

    let summary = format!(
        "Behavioral dead transition: {anchor_label} never fires on any reachable interleaving"
    );

    let trace = enrich_trace(net, cx, fn_to_module);
    let final_marking_summary = format_marking(net, &cx.final_state.marking);
    let kind = BugKind::DeadTransition {
        transition: transition_id_str,
        sids,
    };
    let involved_modules = modules_for_functions(&involved_functions, fn_to_module);
    let repair_hint = suggestion::suggestion_for(&kind);

    BugReport {
        kind,
        trace,
        final_marking_summary,
        summary,
        involved_resources: Vec::new(),
        involved_functions,
        involved_modules,
        cir_slice: Vec::new(),
        preservation_constraints: Vec::new(),
        repair_hint,
    }
}

fn classify_deadlock(
    net: &CvnNet,
    cx: &Counterexample,
    blocked: &[PlaceId],
    fn_to_module: &HashMap<String, String>,
) -> (BugKind, String) {
    let participants = analyze_deadlock_participants(net, cx, blocked, fn_to_module);

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
    fn_to_module: &HashMap<String, String>,
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
            module: fn_to_module.get(&fn_name).cloned(),
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

fn enrich_trace(
    net: &CvnNet,
    cx: &Counterexample,
    fn_to_module: &HashMap<String, String>,
) -> Vec<EnrichedFiringStep> {
    cx.trace
        .iter()
        .map(|step| {
            let transition = net.transition(&step.transition_id);
            let kind = transition
                .map(|t| t.kind.clone())
                .unwrap_or(cvn::model::TransitionKind::Sequential);
            let source_function = transition
                .and_then(|t| t.source_function.clone());
            let module = source_function
                .as_ref()
                .and_then(|f| fn_to_module.get(f).cloned());

            let anchor_sids: Vec<String> = step.anchor_sids.iter().cloned().collect();

            let description =
                format_step_description(&step.transition_id, &kind, &anchor_sids, &source_function);

            EnrichedFiringStep {
                transition_id: step.transition_id.0.clone(),
                kind,
                anchor_sids,
                source_function,
                module,
                description,
            }
        })
        .collect()
}

fn format_step_description(
    tid: &TransitionId,
    kind: &cvn::model::TransitionKind,
    anchor_sids: &[String],
    source_function: &Option<String>,
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
        match source_function {
            Some(fn_name) => format!("{kind_str} ({}) — in {fn_name}", tid.0),
            None => format!("{kind_str} ({})", tid.0),
        }
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

/// Extract ConcIR statements relevant to the bug trace (Lambda in the diagnostic tuple).
///
/// SIDs are only unique within a function, so attribution uses the
/// (function, sid) pair: `source_function` (when present) scopes the SID
/// lookup, otherwise every function's statements are considered.
fn extract_cir_slice(
    program: &concir::ast::Program,
    trace: &[report::EnrichedFiringStep],
) -> Vec<report::CirSliceEntry> {
    let mut scoped: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut unscoped: HashSet<&str> = HashSet::new();

    for step in trace {
        match &step.source_function {
            Some(fn_name) => scoped
                .entry(fn_name.as_str())
                .or_default()
                .extend(step.anchor_sids.iter().map(String::as_str)),
            None => unscoped.extend(step.anchor_sids.iter().map(String::as_str)),
        }
    }

    let mut entries = Vec::new();
    for func in &program.functions {
        let scoped_sids = scoped
            .get(func.name.as_str())
            .cloned()
            .unwrap_or_default();
        for stmt in &func.body {
            let in_scope = scoped_sids.contains(stmt.sid.as_str())
                || (unscoped.contains(stmt.sid.as_str()) && scoped_sids.is_empty());
            if in_scope {
                entries.push(report::CirSliceEntry {
                    sid: stmt.sid.clone(),
                    op: format!("{:?}", stmt.op),
                    function: func.name.clone(),
                    module: func.module.clone(),
                });
            }
        }
    }
    entries
}

/// Build preservation constraints from the ConcIR program (Gamma_ctx).
pub(crate) fn build_preservation_constraints(program: &concir::ast::Program) -> Vec<String> {
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
