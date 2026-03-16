use crate::common;
use cvn::model::{PlaceId, TransitionId, TransitionKind};

#[test]
fn spawn_creates_fork() {
    let net = common::translate_fixture("spawn_join.json");

    let tid = TransitionId::new("main_s1_spawn");
    let t = net.transition(&tid).unwrap();
    assert!(matches!(t.kind, TransitionKind::Spawn));

    let out = net.output_arcs(&tid);
    // Should produce two tokens: one to cp_main_s2, one to worker's first place.
    assert_eq!(out.len(), 2);
    assert!(out.iter().any(|a| a.place == PlaceId::new("cp_main_s2")));
}

#[test]
fn join_creates_synchronization() {
    let net = common::translate_fixture("spawn_join.json");

    let tid = TransitionId::new("main_s2_join");
    let t = net.transition(&tid).unwrap();
    assert!(matches!(t.kind, TransitionKind::Join));

    let in_arcs = net.input_arcs(&tid);
    // Should consume from cp_main_s2 AND cp_worker_ret.
    assert_eq!(in_arcs.len(), 2);
    assert!(in_arcs.iter().any(|a| a.place == PlaceId::new("cp_main_s2")));
    assert!(in_arcs.iter().any(|a| a.place == PlaceId::new("cp_worker_ret")));
}

#[test]
fn spawn_join_initial_marking() {
    let net = common::translate_fixture("spawn_join.json");

    assert_eq!(common::initial_tokens(&net, "cp_main_s1"), 1);
    assert_eq!(common::initial_tokens(&net, "cp_worker_s1"), 0);
}
