//! Budgeted composite patch search.
//!
//! Three comparable strategies share the same edit space, permissions,
//! verification semantics, and budgets:
//!
//! - `Single` (A): expand only the root, one edit deep — the legacy baseline.
//! - `Composite` (B): bounded BFS over nodes, no diagnostic guidance.
//! - `Diagnostic` (C): the same BFS, but a node's structured diagnostics
//!   (blocked/holder resource facts) filter the candidate edits.
//!
//! The frozen `ContractSpec` is re-resolved against every candidate program and
//! fully verified. Intermediate `FAIL` nodes are kept as search nodes; only an
//! overall complete `PASS` is accepted. Stopping for any reason never claims
//! that no repair exists.

use std::collections::{BTreeSet, VecDeque};

use serde::Serialize;

use crate::ast::Program;
use crate::explore::contract::ContractSpec;
use crate::explore::{verify_program, EngineKind, VerificationReport};
use crate::sem::outcome::{AnalysisBounds, Outcome};
use crate::validate;

use super::candidates::{
    CandidateProvider, LockOrderCompositeEnumerator, LockOrderEnumerator, NodeHistory, RepairContext,
};
use super::patch::{self, CirPatch, PatchChange, SourceRelation};
use super::RepairOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairStrategy {
    /// A: single, diagnostic-free enumeration on the original program.
    Single,
    /// B: bounded composite search, no diagnostic guidance.
    Composite,
    /// C: bounded composite search guided by diagnostics.
    Diagnostic,
}

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub strategy: RepairStrategy,
    pub candidate_budget: usize,
    pub verification_budget: usize,
    pub max_depth: usize,
    pub max_total_edits: usize,
    pub bounds: AnalysisBounds,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            strategy: RepairStrategy::Diagnostic,
            candidate_budget: 64,
            verification_budget: 64,
            max_depth: 4,
            max_total_edits: 4,
            bounds: AnalysisBounds {
                max_states: 20_000,
                max_depth: 64,
                ..AnalysisBounds::default()
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AppliedEdit {
    pub module: String,
    pub function: String,
    pub changes: Vec<PatchChange>,
    pub provenance: Vec<SourceRelation>,
    pub parent_fingerprint: String,
    pub program_fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeReport {
    pub id: usize,
    pub parent: Option<usize>,
    pub depth: usize,
    pub total_edits: usize,
    pub fingerprint: String,
    pub outcome: Outcome,
    pub complete: bool,
    pub states_explored: usize,
    pub properties: Vec<(String, Outcome)>,
    /// Why this node was not expanded (if any).
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchReport {
    pub strategy: RepairStrategy,
    pub outcome: RepairOutcome,
    pub stop_reason: String,
    pub candidates_tried: usize,
    pub verifications: usize,
    pub states_explored: usize,
    pub patch_chain: Vec<AppliedEdit>,
    pub nodes: Vec<NodeReport>,
    #[serde(skip)]
    pub accepted_program: Option<Program>,
    #[serde(skip)]
    pub accepted_report: Option<VerificationReport>,
}

impl SearchReport {
    pub fn accepted_patch_count(&self) -> usize {
        self.patch_chain.len()
    }
}

pub fn program_fingerprint(program: &Program) -> String {
    let s = serde_json::to_string(program).unwrap_or_default();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

struct Node {
    parent: Option<usize>,
    depth: usize,
    total_edits: usize,
    program: Program,
    fingerprint: String,
    report: VerificationReport,
    incoming: Option<CirPatch>,
}

fn early_report(
    config: &SearchConfig,
    outcome: RepairOutcome,
    stop_reason: &str,
    verifications: usize,
    states_explored: usize,
) -> SearchReport {
    SearchReport {
        strategy: config.strategy,
        outcome,
        stop_reason: stop_reason.to_string(),
        candidates_tried: 0,
        verifications,
        states_explored,
        patch_chain: Vec::new(),
        nodes: Vec::new(),
        accepted_program: None,
        accepted_report: None,
    }
}

fn outcome_of(report: &VerificationReport) -> Outcome {
    report.outcome
}

fn properties(report: &VerificationReport) -> Vec<(String, Outcome)> {
    report
        .properties
        .iter()
        .map(|p| (p.id.clone(), p.outcome))
        .collect()
}

/// Run the budgeted search. Deterministic for a fixed input and budget.
pub fn run_search(program: &Program, spec: &ContractSpec, config: &SearchConfig) -> SearchReport {
    let mut nodes: Vec<Node> = Vec::new();
    let mut records: Vec<NodeReport> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut candidates_tried = 0usize;
    let mut verifications = 0usize;
    let mut states_explored = 0usize;
    let mut budget_hit = false;
    let mut saw_unknown = false;

    let root_fp = program_fingerprint(program);
    let root_report = verify_program(program, spec, EngineKind::Petri);
    verifications += 1;
    states_explored += root_report.states_explored;

    // 1. Verify the original program first.
    match outcome_of(&root_report) {
        Outcome::Pass if root_report.complete => {
            let mut r = SearchReport {
                strategy: config.strategy,
                outcome: RepairOutcome::AlreadySatisfied,
                stop_reason: "already-satisfied".into(),
                candidates_tried,
                verifications,
                states_explored,
                patch_chain: Vec::new(),
                nodes: Vec::new(),
                accepted_program: None,
                accepted_report: None,
            };
            r.nodes = vec![NodeReport {
                id: 0,
                parent: None,
                depth: 0,
                total_edits: 0,
                fingerprint: root_fp,
                outcome: Outcome::Pass,
                complete: true,
                states_explored: root_report.states_explored,
                properties: properties(&root_report),
                note: Some("contract already satisfied; no patch needed".into()),
            }];
            return r;
        }
        Outcome::Invalid => return early_report(config, RepairOutcome::Invalid, "root-invalid", verifications, states_explored),
        Outcome::Unsupported => return early_report(config, RepairOutcome::Unsupported, "root-unsupported", verifications, states_explored),
        Outcome::Unknown => return early_report(config, RepairOutcome::AnalysisUnknown, "root-unknown", verifications, states_explored),
        _ => {}
    }

    seen.insert(root_fp.clone());
    nodes.push(Node {
        parent: None,
        depth: 0,
        total_edits: 0,
        program: program.clone(),
        fingerprint: root_fp.clone(),
        report: root_report.clone(),
        incoming: None,
    });
    records.push(NodeReport {
        id: 0,
        parent: None,
        depth: 0,
        total_edits: 0,
        fingerprint: root_fp,
        outcome: root_report.outcome,
        complete: root_report.complete,
        states_explored: root_report.states_explored,
        properties: properties(&root_report),
        note: None,
    });

    let mut queue: VecDeque<usize> = VecDeque::from([0usize]);

    'outer: while let Some(nid) = queue.pop_front() {
        let (node_program, node_report, node_depth, node_edits, node_fp) = {
            let n = &nodes[nid];
            (
                n.program.clone(),
                n.report.clone(),
                n.depth,
                n.total_edits,
                n.fingerprint.clone(),
            )
        };
        if node_depth >= config.max_depth || node_edits >= config.max_total_edits {
            continue;
        }
        let scope = &spec.allowed_scope;
        let mut provider: Box<dyn CandidateProvider> = match config.strategy {
            RepairStrategy::Single => Box::new(LockOrderEnumerator::new(&node_program, scope)),
            RepairStrategy::Composite => {
                Box::new(LockOrderCompositeEnumerator::new(&node_program, scope, false))
            }
            RepairStrategy::Diagnostic => {
                Box::new(LockOrderCompositeEnumerator::new(&node_program, scope, true))
            }
        };
        let history: Vec<NodeHistory> = ancestor_history(&nodes, nid);

        loop {
            if candidates_tried >= config.candidate_budget {
                budget_hit = true;
                break 'outer;
            }
            let ctx = RepairContext {
                program: &node_program,
                spec,
                round: node_depth,
                depth: node_depth,
                report: Some(&node_report),
                history: &history,
            };
            let Some(candidate) = provider.next_candidate(&ctx) else {
                break;
            };
            candidates_tried += 1;

            if let Err(e) = patch::check_allowed(scope, &candidate) {
                records.push(rejected_record(
                    &candidate,
                    node_depth + 1,
                    node_edits + candidate.changes.len(),
                    format!("disallowed: {e}"),
                ));
                continue;
            }
            let patched = match patch::apply(&node_program, &candidate) {
                Ok((p, _diff)) => p,
                Err(e) => {
                    records.push(rejected_record(
                        &candidate,
                        node_depth + 1,
                        node_edits + candidate.changes.len(),
                        format!("patch rejected: {e}"),
                    ));
                    continue;
                }
            };
            if !validate::validate(&patched).valid {
                records.push(rejected_record(
                    &candidate,
                    node_depth + 1,
                    node_edits + candidate.changes.len(),
                    "static validation failed".into(),
                ));
                continue;
            }
            if verifications >= config.verification_budget {
                budget_hit = true;
                break 'outer;
            }
            let child_report = verify_program(&patched, spec, EngineKind::Petri);
            verifications += 1;
            states_explored += child_report.states_explored;
            let child_fp = program_fingerprint(&patched);
            let child_depth = node_depth + 1;
            let child_edits = node_edits + candidate.changes.len();
            let accepted = child_report.outcome == Outcome::Pass && child_report.complete;

            records.push(NodeReport {
                id: records.len(),
                parent: Some(nid),
                depth: child_depth,
                total_edits: child_edits,
                fingerprint: child_fp.clone(),
                outcome: child_report.outcome,
                complete: child_report.complete,
                states_explored: child_report.states_explored,
                properties: properties(&child_report),
                note: if accepted {
                    Some("accepted".into())
                } else {
                    None
                },
            });

            if accepted {
                let chain = build_chain(&nodes, nid, &candidate, &node_fp, &child_fp);
                return SearchReport {
                    strategy: config.strategy,
                    outcome: RepairOutcome::Repaired,
                    stop_reason: "solved".into(),
                    candidates_tried,
                    verifications,
                    states_explored,
                    patch_chain: chain,
                    nodes: records,
                    accepted_program: Some(patched),
                    accepted_report: Some(child_report),
                };
            }

            match child_report.outcome {
                Outcome::Fail => {
                    if !seen.contains(&child_fp) {
                        seen.insert(child_fp.clone());
                        let id = nodes.len();
                        nodes.push(Node {
                            parent: Some(nid),
                            depth: child_depth,
                            total_edits: child_edits,
                            program: patched,
                            fingerprint: child_fp,
                            report: child_report,
                            incoming: Some(candidate),
                        });
                        if config.strategy != RepairStrategy::Single {
                            queue.push_back(id);
                        }
                    }
                }
                Outcome::Unknown => {
                    saw_unknown = true;
                }
                _ => {}
            }
        }
    }

    let outcome = if budget_hit {
        RepairOutcome::BudgetExhausted
    } else if saw_unknown {
        RepairOutcome::AnalysisUnknown
    } else {
        RepairOutcome::NoAcceptableCandidate
    };
    let stop_reason = if budget_hit {
        "budget-exhausted"
    } else if saw_unknown {
        "analysis-unknown"
    } else {
        "no-acceptable-candidate"
    };
    SearchReport {
        strategy: config.strategy,
        outcome,
        stop_reason: stop_reason.into(),
        candidates_tried,
        verifications,
        states_explored,
        patch_chain: Vec::new(),
        nodes: records,
        accepted_program: None,
        accepted_report: None,
    }
}

fn rejected_record(c: &CirPatch, depth: usize, edits: usize, note: String) -> NodeReport {
    NodeReport {
        id: 0,
        parent: None,
        depth,
        total_edits: edits,
        fingerprint: format!("rejected:{}", c.id),
        outcome: Outcome::Invalid,
        complete: false,
        states_explored: 0,
        properties: Vec::new(),
        note: Some(note),
    }
}

fn ancestor_history(nodes: &[Node], nid: usize) -> Vec<NodeHistory> {
    let mut chain = Vec::new();
    let mut cur = Some(nid);
    while let Some(id) = cur {
        let n = &nodes[id];
        chain.push(id);
        cur = n.parent;
    }
    chain.reverse();
    chain
        .into_iter()
        .map(|id| {
            let n = &nodes[id];
            NodeHistory {
                depth: n.depth,
                edits: n
                    .incoming
                    .as_ref()
                    .map(|c| format!("{}:{}:{:?}", c.module, c.function, c.changes))
                    .into_iter()
                    .collect(),
                outcome: n.report.outcome,
            }
        })
        .collect()
}

fn build_chain(
    nodes: &[Node],
    parent: usize,
    candidate: &CirPatch,
    parent_fp: &str,
    child_fp: &str,
) -> Vec<AppliedEdit> {
    let mut chain: Vec<AppliedEdit> = Vec::new();
    let mut cur = Some(parent);
    while let Some(id) = cur {
        let n = &nodes[id];
        if let Some(inc) = &n.incoming {
            chain.push(AppliedEdit {
                module: inc.module.clone(),
                function: inc.function.clone(),
                changes: inc.changes.clone(),
                provenance: inc.provenance.clone(),
                parent_fingerprint: n
                    .parent
                    .map(|p| nodes[p].fingerprint.clone())
                    .unwrap_or_default(),
                program_fingerprint: n.fingerprint.clone(),
            });
        }
        cur = n.parent;
    }
    chain.reverse();
    chain.push(AppliedEdit {
        module: candidate.module.clone(),
        function: candidate.function.clone(),
        changes: candidate.changes.clone(),
        provenance: candidate.provenance.clone(),
        parent_fingerprint: parent_fp.to_string(),
        program_fingerprint: child_fp.to_string(),
    });
    chain
}
