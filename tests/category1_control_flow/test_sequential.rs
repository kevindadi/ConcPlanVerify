use crate::common;
use cvn::model::{PlaceId, TransitionId};

#[test]
fn sequential_chain_places() {
    let net = common::translate_fixture("sequential_chain.json");

    assert!(common::has_place(&net, "cp_worker_s1"));
    assert!(common::has_place(&net, "cp_worker_s2"));
    assert!(common::has_place(&net, "cp_worker_s3"));
    assert!(common::has_place(&net, "cp_worker_ret"));
    assert!(common::has_place(&net, "rp_mtx"));
}

#[test]
fn sequential_chain_transitions() {
    let net = common::translate_fixture("sequential_chain.json");

    // s1: lock, s2: unlock, s3: return
    assert!(net.transition(&TransitionId::new("worker_s1_lock")).is_some());
    assert!(net.transition(&TransitionId::new("worker_s2_unlock")).is_some());
    assert!(net.transition(&TransitionId::new("worker_s3_return")).is_some());
}

#[test]
fn sequential_chain_initial_marking() {
    let net = common::translate_fixture("sequential_chain.json");

    assert_eq!(common::initial_tokens(&net, "cp_worker_s1"), 1);
    assert_eq!(common::initial_tokens(&net, "rp_mtx"), 1);
    assert_eq!(common::initial_tokens(&net, "cp_worker_s2"), 0);
}

#[test]
fn sequential_chain_lock_arcs() {
    let net = common::translate_fixture("sequential_chain.json");

    let tid = TransitionId::new("worker_s1_lock");
    let in_arcs = net.input_arcs(&tid);
    assert_eq!(in_arcs.len(), 2);

    let cp_arc = in_arcs.iter().find(|a| a.place == PlaceId::new("cp_worker_s1")).unwrap();
    assert_eq!(cp_arc.weight, 1);

    let rp_arc = in_arcs.iter().find(|a| a.place == PlaceId::new("rp_mtx")).unwrap();
    assert_eq!(rp_arc.weight, 1);

    let out_arcs = net.output_arcs(&tid);
    assert_eq!(out_arcs.len(), 1);
    assert_eq!(out_arcs[0].place, PlaceId::new("cp_worker_s2"));
}

#[test]
fn sequential_chain_unlock_arcs() {
    let net = common::translate_fixture("sequential_chain.json");

    let tid = TransitionId::new("worker_s2_unlock");
    let out_arcs = net.output_arcs(&tid);
    assert_eq!(out_arcs.len(), 2);

    assert!(out_arcs.iter().any(|a| a.place == PlaceId::new("cp_worker_s3")));
    assert!(out_arcs.iter().any(|a| a.place == PlaceId::new("rp_mtx")));
}
