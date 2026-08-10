#![allow(dead_code)]
use std::path::Path;

use concir::ast::Program;
use unipn::model::TransitionKind;
use unipn::{Net, NetLike, PlaceId, TransitionId, VarUpdate};

pub fn load_fixture(name: &str) -> Program {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to parse fixture {}: {e}", path.display()))
}

pub fn translate_fixture(name: &str) -> Net {
    let program = load_fixture(name);
    cir2cvn::translate(&program).unwrap_or_else(|errs| {
        panic!(
            "translation failed for {name}: {:?}",
            errs.iter().map(|e| e.to_string()).collect::<Vec<_>>()
        )
    })
}

/// Look up a place by its `Place::name` (e.g. `"main.s5"`, `"mtx"`).
pub fn place_by_name(net: &Net, name: &str) -> Option<PlaceId> {
    net.places().iter().find(|p| p.name == name).map(|p| p.id)
}

/// Look up a transition by its `Transition::name` (e.g. `"main_s5_branch_true"`).
pub fn transition_by_name(net: &Net, name: &str) -> Option<TransitionId> {
    net.transitions()
        .iter()
        .find(|t| t.name == name)
        .map(|t| t.id)
}

pub fn has_place(net: &Net, name: &str) -> bool {
    place_by_name(net, name).is_some()
}

pub fn initial_tokens(net: &Net, name: &str) -> u32 {
    match place_by_name(net, name) {
        Some(pid) => net.initial_state().marking.tokens(pid),
        None => 0,
    }
}

pub fn transition_count(net: &Net) -> usize {
    net.num_transitions()
}

pub fn place_count(net: &Net) -> usize {
    net.num_places()
}

/// The guard of the input arc `place → transition`, if any.
pub fn input_guard(
    net: &Net,
    transition: TransitionId,
    place: PlaceId,
) -> Option<unipn::BoolExpr> {
    net.input_guard(transition, place).cloned()
}

/// The variable update of the output arc `transition → place`, if any.
pub fn output_update(
    net: &Net,
    transition: TransitionId,
    place: PlaceId,
) -> Option<VarUpdate> {
    net.output_update(transition, place).cloned()
}

pub fn transition_kind(net: &Net, name: &str) -> Option<TransitionKind> {
    transition_by_name(net, name).and_then(|t| net.transition_kind(t))
}

/// Convenience: build a single-var update.
pub fn update(var: &str, expr: unipn::Expr) -> Option<VarUpdate> {
    let mut u = VarUpdate::new();
    u.insert(var.to_string(), expr);
    Some(u)
}

/// The `(place name, weight)` pairs of a transition's input arcs.
pub fn input_arcs(net: &Net, transition: &str) -> Vec<(String, Weight)> {
    let tid = transition_by_name(net, transition).expect("transition not found");
    net.pre_arcs(tid)
        .iter()
        .map(|(p, w)| (place_name(net, *p), *w))
        .collect()
}

/// The `(place name, weight)` pairs of a transition's output arcs.
pub fn output_arcs(net: &Net, transition: &str) -> Vec<(String, Weight)> {
    let tid = transition_by_name(net, transition).expect("transition not found");
    net.post_arcs(tid)
        .iter()
        .map(|(p, w)| (place_name(net, *p), *w))
        .collect()
}

pub fn place_name(net: &Net, p: PlaceId) -> String {
    net.place(p).map(|pl| pl.name.clone()).unwrap_or_default()
}

pub fn has_input_arc(net: &Net, transition: &str, place: &str) -> bool {
    input_arcs(net, transition).iter().any(|(n, _)| n == place)
}

pub fn has_output_arc(net: &Net, transition: &str, place: &str) -> bool {
    output_arcs(net, transition).iter().any(|(n, _)| n == place)
}

/// The guard on `place → transition`, looked up by names.
pub fn input_guard_by_name(net: &Net, transition: &str, place: &str) -> Option<unipn::BoolExpr> {
    let t = transition_by_name(net, transition)?;
    let p = place_by_name(net, place)?;
    net.input_guard(t, p).cloned()
}

/// The update on `transition → place`, looked up by names.
pub fn output_update_by_name(net: &Net, transition: &str, place: &str) -> Option<VarUpdate> {
    let t = transition_by_name(net, transition)?;
    let p = place_by_name(net, place)?;
    net.output_update(t, p).cloned()
}

/// Whether a transition with the given name exists.
pub fn has_transition(net: &Net, name: &str) -> bool {
    transition_by_name(net, name).is_some()
}

pub type Weight = u32;

/// The initial variable store (empty when the net models no data).
pub fn initial_vars(net: &Net) -> unipn::VarStore {
    net.initial_state().vars.clone().unwrap_or_default()
}

/// The source scope of a transition (by name).
pub fn transition_scope(net: &Net, name: &str) -> Option<String> {
    transition_by_name(net, name).and_then(|t| net.transition_scope(t).map(String::from))
}
