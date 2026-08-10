use crate::common;

#[test]
fn mutex_resource_place_exists() {
    let net = common::translate_fixture("mutex_exclusive.json");
    assert!(common::has_place(&net, "mtx"));
    assert_eq!(common::initial_tokens(&net, "mtx"), 1);
}

#[test]
fn mutex_lock_consumes_token() {
    let net = common::translate_fixture("mutex_exclusive.json");

    let in_arcs = common::input_arcs(&net, "w1_s1_lock");
    assert!(in_arcs.iter().any(|(n, w)| n == "mtx" && *w == 1));
}

#[test]
fn mutex_unlock_produces_token() {
    let net = common::translate_fixture("mutex_exclusive.json");

    let out_arcs = common::output_arcs(&net, "w1_s2_unlock");
    assert!(out_arcs.iter().any(|(n, w)| n == "mtx" && *w == 1));
}

#[test]
fn mutex_two_workers_conflict() {
    let net = common::translate_fixture("mutex_exclusive.json");

    // Both w1_s1_lock and w2_s1_lock consume from mtx.
    assert!(common::has_input_arc(&net, "w1_s1_lock", "mtx"));
    assert!(common::has_input_arc(&net, "w2_s1_lock", "mtx"));
}
