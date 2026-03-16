use crate::common;
use cvn::model::{TransitionId, TransitionKind};

#[test]
fn cas_creates_success_and_failure() {
    let net = common::translate_fixture("cas.json");

    let t_succ = net.transition(&TransitionId::new("main_s1_branch_true")).unwrap();
    let t_fail = net.transition(&TransitionId::new("main_s1_branch_false")).unwrap();

    assert!(matches!(t_succ.kind, TransitionKind::CasSuccess));
    assert!(matches!(t_fail.kind, TransitionKind::CasFailure));
}

#[test]
fn cas_success_has_update() {
    let net = common::translate_fixture("cas.json");

    let tid = TransitionId::new("main_s1_branch_true");
    let out = net.output_arcs(&tid);
    assert!(!out.is_empty());
    let update = out[0].update.as_ref().expect("CAS success should have update");
    assert!(update.contains_key("flag"));
}

#[test]
fn cas_failure_no_update() {
    let net = common::translate_fixture("cas.json");

    let tid = TransitionId::new("main_s1_branch_false");
    let out = net.output_arcs(&tid);
    assert!(!out.is_empty());
    assert!(out[0].update.is_none());
}
