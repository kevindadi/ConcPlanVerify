use crate::common;
use unipn::model::TransitionKind;

#[test]
fn spawn_creates_fork() {
    let net = common::translate_fixture("spawn_join.json");

    assert!(common::transition_kind(&net, "main_s1_spawn").is_some_and(|k| k == TransitionKind::Spawn));

    let out = common::output_arcs(&net, "main_s1_spawn");
    // Should produce two tokens: one to main.s2, one to worker's first place.
    assert_eq!(out.len(), 2);
    assert!(out.iter().any(|(n, _)| n == "main.s2"));
}

#[test]
fn join_creates_synchronization() {
    let net = common::translate_fixture("spawn_join.json");

    assert!(common::transition_kind(&net, "main_s2_join").is_some_and(|k| k == TransitionKind::Join));

    let in_arcs = common::input_arcs(&net, "main_s2_join");
    // Should consume from main.s2 AND worker.ret.
    assert_eq!(in_arcs.len(), 2);
    assert!(in_arcs.iter().any(|(n, _)| n == "main.s2"));
    assert!(in_arcs.iter().any(|(n, _)| n == "worker.ret"));
}

#[test]
fn spawn_join_initial_marking() {
    let net = common::translate_fixture("spawn_join.json");

    assert_eq!(common::initial_tokens(&net, "main.s1"), 1);
    assert_eq!(common::initial_tokens(&net, "worker.s1"), 0);
}
