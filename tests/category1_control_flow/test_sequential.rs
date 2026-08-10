use crate::common;

#[test]
fn sequential_chain_places() {
    let net = common::translate_fixture("sequential_chain.json");

    assert!(common::has_place(&net, "worker.s1"));
    assert!(common::has_place(&net, "worker.s2"));
    assert!(common::has_place(&net, "worker.s3"));
    assert!(common::has_place(&net, "worker.ret"));
    assert!(common::has_place(&net, "mtx"));
}

#[test]
fn sequential_chain_transitions() {
    let net = common::translate_fixture("sequential_chain.json");

    // s1: lock, s2: unlock, s3: return
    assert!(common::has_transition(&net, "worker_s1_lock"));
    assert!(common::has_transition(&net, "worker_s2_unlock"));
    assert!(common::has_transition(&net, "worker_s3_return"));
}

#[test]
fn sequential_chain_initial_marking() {
    let net = common::translate_fixture("sequential_chain.json");

    assert_eq!(common::initial_tokens(&net, "worker.s1"), 1);
    assert_eq!(common::initial_tokens(&net, "mtx"), 1);
    assert_eq!(common::initial_tokens(&net, "worker.s2"), 0);
}

#[test]
fn sequential_chain_lock_arcs() {
    let net = common::translate_fixture("sequential_chain.json");

    let in_arcs = common::input_arcs(&net, "worker_s1_lock");
    assert_eq!(in_arcs.len(), 2);
    assert!(in_arcs.iter().any(|(n, w)| n == "worker.s1" && *w == 1));
    assert!(in_arcs.iter().any(|(n, w)| n == "mtx" && *w == 1));

    let out_arcs = common::output_arcs(&net, "worker_s1_lock");
    assert_eq!(out_arcs.len(), 1);
    assert_eq!(out_arcs[0].0, "worker.s2");
}

#[test]
fn sequential_chain_unlock_arcs() {
    let net = common::translate_fixture("sequential_chain.json");

    let out_arcs = common::output_arcs(&net, "worker_s2_unlock");
    assert_eq!(out_arcs.len(), 2);
    assert!(out_arcs.iter().any(|(n, _)| n == "worker.s3"));
    assert!(out_arcs.iter().any(|(n, _)| n == "mtx"));
}
