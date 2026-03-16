use crate::common;
use cvn::model::{PlaceId, TransitionId};

#[test]
fn mutex_resource_place_exists() {
    let net = common::translate_fixture("mutex_exclusive.json");
    assert!(common::has_place(&net, "rp_mtx"));
    assert_eq!(common::initial_tokens(&net, "rp_mtx"), 1);
}

#[test]
fn mutex_lock_consumes_token() {
    let net = common::translate_fixture("mutex_exclusive.json");

    let tid = TransitionId::new("w1_s1_lock");
    let in_arcs = net.input_arcs(&tid);
    let rp_arc = in_arcs.iter().find(|a| a.place == PlaceId::new("rp_mtx")).unwrap();
    assert_eq!(rp_arc.weight, 1);
}

#[test]
fn mutex_unlock_produces_token() {
    let net = common::translate_fixture("mutex_exclusive.json");

    let tid = TransitionId::new("w1_s2_unlock");
    let out_arcs = net.output_arcs(&tid);
    let rp_arc = out_arcs.iter().find(|a| a.place == PlaceId::new("rp_mtx")).unwrap();
    assert_eq!(rp_arc.weight, 1);
}

#[test]
fn mutex_two_workers_conflict() {
    let net = common::translate_fixture("mutex_exclusive.json");

    // Both w1_s1_lock and w2_s1_lock consume from rp_mtx.
    let w1_in = net.input_arcs(&TransitionId::new("w1_s1_lock"));
    let w2_in = net.input_arcs(&TransitionId::new("w2_s1_lock"));
    assert!(w1_in.iter().any(|a| a.place == PlaceId::new("rp_mtx")));
    assert!(w2_in.iter().any(|a| a.place == PlaceId::new("rp_mtx")));
}
