#![allow(dead_code)]
use std::path::Path;

use concir::ast::Program;
use unipn::model::TransitionKind;
use unipn::net::ArcDir;
use unipn::{CvnArcKind, CvnNet, CvnState, PlaceId, TransitionId, VarUpdate};

/// A translated program: the net plus its initial state.
pub struct Fixture {
    pub net: CvnNet,
    pub initial: CvnState,
}

impl std::ops::Deref for Fixture {
    type Target = CvnNet;

    fn deref(&self) -> &Self::Target {
        &self.net
    }
}

pub fn load_fixture(name: &str) -> Program {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to parse fixture {}: {e}", path.display()))
}

pub fn translate_program(program: &Program) -> Fixture {
    let (net, initial) = cir2cvn::translate(program).unwrap_or_else(|errs| {
        panic!(
            "translation failed: {:?}",
            errs.iter().map(|e| e.to_string()).collect::<Vec<_>>()
        )
    });
    Fixture { net, initial }
}

pub fn translate_fixture(name: &str) -> Fixture {
    translate_program(&load_fixture(name))
}

/// Look up a place by its `Place::name` (e.g. `"main.s5"`, `"mtx"`).
pub fn place_by_name(fx: &Fixture, name: &str) -> Option<PlaceId> {
    fx.net.places.iter().find(|p| p.name == name).map(|p| p.id)
}

/// Look up a transition by its `Transition::name`.
pub fn transition_by_name(fx: &Fixture, name: &str) -> Option<TransitionId> {
    fx.net
        .transitions
        .iter()
        .find(|t| t.name == name)
        .map(|t| t.id)
}

pub fn has_place(fx: &Fixture, name: &str) -> bool {
    place_by_name(fx, name).is_some()
}

pub fn initial_tokens(fx: &Fixture, name: &str) -> usize {
    match place_by_name(fx, name) {
        Some(pid) => fx.initial.marking.tokens(pid),
        None => 0,
    }
}

pub fn transition_count(fx: &Fixture) -> usize {
    fx.net.num_transitions()
}

pub fn place_count(fx: &Fixture) -> usize {
    fx.net.num_places()
}

/// The guard of the input arc `place → transition`, if any.
pub fn input_guard(
    fx: &Fixture,
    transition: TransitionId,
    place: PlaceId,
) -> Option<unipn::BoolExpr> {
    fx.net
        .arcs_of(transition, ArcDir::Input)
        .find(|arc| arc.place == place)
        .and_then(|arc| match &arc.kind {
            CvnArcKind::Guard(g) => Some(g.clone()),
            _ => None,
        })
}

/// The variable update of the output arc `transition → place`, if any.
pub fn output_update(fx: &Fixture, transition: TransitionId, place: PlaceId) -> Option<VarUpdate> {
    fx.net
        .arcs_of(transition, ArcDir::Output)
        .find(|arc| arc.place == place)
        .and_then(|arc| match &arc.kind {
            CvnArcKind::Update(u) => Some(u.clone()),
            _ => None,
        })
}

pub fn transition_kind(fx: &Fixture, name: &str) -> Option<TransitionKind> {
    transition_by_name(fx, name)
        .and_then(|t| fx.net.transition(t))
        .map(|tr| tr.kind.kind.clone())
}

/// Convenience: build a single-var update.
pub fn update(var: &str, expr: unipn::Expr) -> Option<VarUpdate> {
    let mut u = VarUpdate::new();
    u.insert(var.to_string(), expr);
    Some(u)
}

/// The `(place name, weight)` pairs of a transition's input arcs.
pub fn input_arcs(fx: &Fixture, transition: &str) -> Vec<(String, Weight)> {
    let tid = transition_by_name(fx, transition).expect("transition not found");
    fx.net
        .pre_arcs(tid)
        .iter()
        .map(|arc| (place_name(fx, arc.place), arc.weight))
        .collect()
}

/// The `(place name, weight)` pairs of a transition's output arcs.
pub fn output_arcs(fx: &Fixture, transition: &str) -> Vec<(String, Weight)> {
    let tid = transition_by_name(fx, transition).expect("transition not found");
    fx.net
        .post_arcs(tid)
        .iter()
        .map(|arc| (place_name(fx, arc.place), arc.weight))
        .collect()
}

pub fn place_name(fx: &Fixture, p: PlaceId) -> String {
    fx.net
        .place(p)
        .map(|pl| pl.name.clone())
        .unwrap_or_default()
}

pub fn has_input_arc(fx: &Fixture, transition: &str, place: &str) -> bool {
    input_arcs(fx, transition).iter().any(|(n, _)| n == place)
}

pub fn has_output_arc(fx: &Fixture, transition: &str, place: &str) -> bool {
    output_arcs(fx, transition).iter().any(|(n, _)| n == place)
}

/// The guard on `place → transition`, looked up by names.
pub fn input_guard_by_name(fx: &Fixture, transition: &str, place: &str) -> Option<unipn::BoolExpr> {
    let t = transition_by_name(fx, transition)?;
    let p = place_by_name(fx, place)?;
    input_guard(fx, t, p)
}

/// The update on `transition → place`, looked up by names.
pub fn output_update_by_name(fx: &Fixture, transition: &str, place: &str) -> Option<VarUpdate> {
    let t = transition_by_name(fx, transition)?;
    let p = place_by_name(fx, place)?;
    output_update(fx, t, p)
}

/// Whether a transition with the given name exists.
pub fn has_transition(fx: &Fixture, name: &str) -> bool {
    transition_by_name(fx, name).is_some()
}

pub type Weight = usize;

/// The initial variable store (empty when the net models no data).
pub fn initial_vars(fx: &Fixture) -> unipn::VarStore {
    fx.initial.extra.vars.clone()
}

/// The source scope of a transition (by name).
pub fn transition_scope(fx: &Fixture, name: &str) -> Option<String> {
    transition_by_name(fx, name)
        .and_then(|t| fx.net.transition(t))
        .and_then(|tr| tr.kind.scope.clone())
}
