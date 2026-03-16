use crate::common;
use cvn::model::{PlaceId, TransitionId};

#[test]
fn loop_has_back_edge() {
    let net = common::translate_fixture("loop_back_edge.json");

    // s3 → s1 is the back edge (return op with transfer next s1).
    let tid = TransitionId::new("main_s3_return");
    let t = net.transition(&tid);
    assert!(t.is_some());

    let out = net.output_arcs(&tid);
    // Should point back to cp_main_s1.
    assert!(out.iter().any(|a| a.place == PlaceId::new("cp_main_s1")));
}

#[test]
fn loop_branch_at_s1() {
    let net = common::translate_fixture("loop_back_edge.json");

    let t_enter = net.transition(&TransitionId::new("main_s1_branch_true"));
    let t_exit = net.transition(&TransitionId::new("main_s1_branch_false"));
    assert!(t_enter.is_some());
    assert!(t_exit.is_some());
}

#[test]
fn loop_var_write_update() {
    let net = common::translate_fixture("loop_back_edge.json");

    let tid = TransitionId::new("main_s2_var_write");
    let out = net.output_arcs(&tid);
    assert!(!out.is_empty());

    let update = out[0].update.as_ref().expect("should have update");
    assert!(update.contains_key("i"));
}
