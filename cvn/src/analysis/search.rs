//! State space search engine (BFS/DFS) for CVN analysis.

use crate::analysis::counterexample::{Counterexample, FiringStep, PropertyViolation};
use crate::analysis::deadlock;
use crate::error::{CvnError, ErrorCode, ErrorLocation};
use crate::model::{State, TransitionId};
use crate::net::CvnNet;
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;

/// Search strategy for state space exploration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SearchStrategy {
    /// Breadth-first search (finds shortest counterexamples).
    #[default]
    Bfs,
    /// Depth-first search (lower memory usage).
    Dfs,
}

/// Configuration for the analysis engine.
#[derive(Clone, Debug)]
pub struct AnalysisConfig {
    /// The search strategy to use.
    pub strategy: SearchStrategy,
    /// Maximum number of states to explore before aborting.
    pub max_states: usize,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            strategy: SearchStrategy::Bfs,
            max_states: 100_000,
        }
    }
}

/// Result of a state space analysis.
#[derive(Clone, Debug)]
pub struct AnalysisResult {
    /// The reachability graph: nodes are states, edges are transition IDs.
    pub reachability_graph: DiGraph<State, TransitionId>,
    /// All detected deadlock counterexamples.
    pub deadlocks: Vec<Counterexample>,
    /// Total number of states explored.
    pub state_count: usize,
}

/// Explore the full reachable state space of a CVN network.
///
/// Returns the reachability graph and any deadlocks found.
pub fn explore(net: &CvnNet, config: &AnalysisConfig) -> Result<AnalysisResult, CvnError> {
    match config.strategy {
        SearchStrategy::Bfs => explore_bfs(net, config),
        SearchStrategy::Dfs => explore_dfs(net, config),
    }
}

fn explore_bfs(net: &CvnNet, config: &AnalysisConfig) -> Result<AnalysisResult, CvnError> {
    let mut graph = DiGraph::<State, TransitionId>::new();
    let mut state_to_node: FxHashMap<u64, NodeIndex> = FxHashMap::default();
    let mut deadlocks = Vec::new();
    let mut queue = VecDeque::new();

    // Predecessor tracking: node_index -> (parent_node_index, transition_id)
    let mut predecessors: FxHashMap<NodeIndex, (NodeIndex, TransitionId)> = FxHashMap::default();

    let initial = net.initial_state();
    let initial_hash = hash_state(&initial);
    let initial_node = graph.add_node(initial.clone());
    state_to_node.insert(initial_hash, initial_node);
    queue.push_back(initial_node);

    while let Some(current_node) = queue.pop_front() {
        if graph.node_count() > config.max_states {
            return Err(CvnError::new(
                ErrorCode::V302,
                format!(
                    "state space explosion: exceeded {} states",
                    config.max_states
                ),
                ErrorLocation::None,
            ));
        }

        let current_state = graph[current_node].clone();
        let enabled = net.enabled_transitions(&current_state);

        if enabled.is_empty() && deadlock::is_deadlock(net, &current_state) {
            let trace = reconstruct_trace(&graph, &predecessors, current_node, net);
            deadlocks.push(Counterexample {
                kind: PropertyViolation::Deadlock,
                trace,
                final_state: current_state.clone(),
            });
            continue;
        }

        for tid in &enabled {
            let new_state = net.fire(tid, &current_state).expect("enabled => can fire");
            let new_hash = hash_state(&new_state);

            let target_node = if let Some(&existing) = state_to_node.get(&new_hash) {
                existing
            } else {
                let node = graph.add_node(new_state);
                state_to_node.insert(new_hash, node);
                predecessors.insert(node, (current_node, tid.clone()));
                queue.push_back(node);
                node
            };

            graph.add_edge(current_node, target_node, tid.clone());
        }
    }

    Ok(AnalysisResult {
        state_count: graph.node_count(),
        reachability_graph: graph,
        deadlocks,
    })
}

fn explore_dfs(net: &CvnNet, config: &AnalysisConfig) -> Result<AnalysisResult, CvnError> {
    let mut graph = DiGraph::<State, TransitionId>::new();
    let mut state_to_node: FxHashMap<u64, NodeIndex> = FxHashMap::default();
    let mut deadlocks = Vec::new();
    let mut stack = Vec::new();

    let mut predecessors: FxHashMap<NodeIndex, (NodeIndex, TransitionId)> = FxHashMap::default();

    let initial = net.initial_state();
    let initial_hash = hash_state(&initial);
    let initial_node = graph.add_node(initial.clone());
    state_to_node.insert(initial_hash, initial_node);
    stack.push(initial_node);

    while let Some(current_node) = stack.pop() {
        if graph.node_count() > config.max_states {
            return Err(CvnError::new(
                ErrorCode::V302,
                format!(
                    "state space explosion: exceeded {} states",
                    config.max_states
                ),
                ErrorLocation::None,
            ));
        }

        let current_state = graph[current_node].clone();
        let enabled = net.enabled_transitions(&current_state);

        if enabled.is_empty() && deadlock::is_deadlock(net, &current_state) {
            let trace = reconstruct_trace(&graph, &predecessors, current_node, net);
            deadlocks.push(Counterexample {
                kind: PropertyViolation::Deadlock,
                trace,
                final_state: current_state.clone(),
            });
            continue;
        }

        for tid in &enabled {
            let new_state = net.fire(tid, &current_state).expect("enabled => can fire");
            let new_hash = hash_state(&new_state);

            let target_node = if let Some(&existing) = state_to_node.get(&new_hash) {
                existing
            } else {
                let node = graph.add_node(new_state);
                state_to_node.insert(new_hash, node);
                predecessors.insert(node, (current_node, tid.clone()));
                stack.push(node);
                node
            };

            graph.add_edge(current_node, target_node, tid.clone());
        }
    }

    Ok(AnalysisResult {
        state_count: graph.node_count(),
        reachability_graph: graph,
        deadlocks,
    })
}

fn hash_state(state: &State) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = rustc_hash::FxHasher::default();
    state.hash(&mut hasher);
    hasher.finish()
}

fn reconstruct_trace(
    _graph: &DiGraph<State, TransitionId>,
    predecessors: &FxHashMap<NodeIndex, (NodeIndex, TransitionId)>,
    target: NodeIndex,
    #[cfg(feature = "cir-anchor")] net: &CvnNet,
    #[cfg(not(feature = "cir-anchor"))] _net: &CvnNet,
) -> Vec<FiringStep> {
    let mut path = Vec::new();
    let mut current = target;

    while let Some((parent, tid)) = predecessors.get(&current) {
        path.push(FiringStep {
            transition_id: tid.clone(),
            #[cfg(feature = "cir-anchor")]
            anchor_sids: net
                .transition(tid)
                .map(|t| t.anchor_sids.clone())
                .unwrap_or_default(),
        });
        current = *parent;
    }

    path.reverse();
    path
}

/// Compute the set of behaviorally dead transitions with respect to
/// the explored reachability graph.
///
/// A transition is *behaviorally dead* when it does not appear on any
/// edge of the reachability graph: i.e. no reachable state enables it.
/// Soundness relative to the CIR follows from the forward-simulation
/// theorem (see `paper/sections/properties.tex`): if the anchored CIR
/// statement could fire on any interleaving, the transition would
/// appear on some edge here.
///
/// Returns one [`Counterexample`] per dead transition, with an empty
/// trace and the initial state as placeholder for `final_state`, since
/// dead transitions have no witness.
pub fn find_dead_transitions(net: &CvnNet, result: &AnalysisResult) -> Vec<Counterexample> {
    use rustc_hash::FxHashSet;

    let mut fired: FxHashSet<TransitionId> = FxHashSet::default();
    for edge_idx in result.reachability_graph.edge_indices() {
        if let Some(tid) = result.reachability_graph.edge_weight(edge_idx) {
            fired.insert(tid.clone());
        }
    }

    let initial = net.initial_state();
    let mut dead = Vec::new();
    for t in net.transitions() {
        if fired.contains(&t.id) {
            continue;
        }
        dead.push(Counterexample {
            kind: PropertyViolation::DeadTransition {
                transition_id: t.id.clone(),
                #[cfg(feature = "cir-anchor")]
                anchor_sids: t.anchor_sids.clone(),
            },
            trace: Vec::new(),
            final_state: initial.clone(),
        });
    }
    dead.sort_by(|a, b| dead_transition_key(&a.kind).cmp(&dead_transition_key(&b.kind)));
    dead
}

fn dead_transition_key(kind: &PropertyViolation) -> String {
    match kind {
        PropertyViolation::DeadTransition { transition_id, .. } => transition_id.0.clone(),
        _ => String::new(),
    }
}

/// Check whether any path exists where a specific condition holds.
///
/// Returns `true` if any reachable state satisfies the predicate.
pub fn exists_path(
    net: &CvnNet,
    config: &AnalysisConfig,
    predicate: impl Fn(&State) -> bool,
) -> Result<bool, CvnError> {
    let result = explore(net, config)?;
    for node_idx in result.reachability_graph.node_indices() {
        if predicate(&result.reachability_graph[node_idx]) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod dead_transition_tests {
    use super::*;
    use crate::builder::CvnNetBuilder;
    use crate::model::{BoolExpr, TransitionKind};

    /// Build a net that has a transition `t_dead` whose input place has
    /// zero initial tokens and no producer, plus a live transition
    /// `t_live` that does fire. Expect exactly `t_dead` to be flagged.
    fn net_with_dead_transition() -> CvnNet {
        CvnNetBuilder::new()
            .add_control_place("p0", "main", "s0")
            .add_control_place("p1", "main", "s1")
            .set_return("p1")
            .add_control_place("p_unreached", "main", "s_unreached")
            .add_transition("t_live", TransitionKind::Sequential)
            .add_transition("t_dead", TransitionKind::Sequential)
            .add_input_arc("p0", "t_live", 1, BoolExpr::True)
            .add_output_arc("t_live", "p1", 1, None)
            .add_input_arc("p_unreached", "t_dead", 1, BoolExpr::True)
            .add_output_arc("t_dead", "p1", 1, None)
            .set_initial_tokens("p0", 1)
            .build()
            .unwrap()
    }

    #[test]
    fn reports_transition_never_enabled() {
        let net = net_with_dead_transition();
        let result = explore(&net, &AnalysisConfig::default()).unwrap();
        let dead = find_dead_transitions(&net, &result);
        assert_eq!(dead.len(), 1, "expected exactly one dead transition");
        match &dead[0].kind {
            PropertyViolation::DeadTransition { transition_id, .. } => {
                assert_eq!(transition_id.0, "t_dead");
            }
            other => panic!("expected DeadTransition, got {:?}", other),
        }
    }

    #[test]
    fn live_transition_not_reported_as_dead() {
        let net = net_with_dead_transition();
        let result = explore(&net, &AnalysisConfig::default()).unwrap();
        let dead = find_dead_transitions(&net, &result);
        for ce in &dead {
            if let PropertyViolation::DeadTransition { transition_id, .. } = &ce.kind {
                assert_ne!(
                    transition_id.0, "t_live",
                    "t_live fires, must not be reported as dead"
                );
            }
        }
    }
}
