use crate::common;
use unipn::model::TransitionKind;

#[test]
fn cas_creates_success_and_failure() {
    let net = common::translate_fixture("cas.json");

    let k_succ = common::transition_kind(&net, "main_s1_branch_true").unwrap();
    let k_fail = common::transition_kind(&net, "main_s1_branch_false").unwrap();
    assert_eq!(k_succ, TransitionKind::CasSuccess);
    assert_eq!(k_fail, TransitionKind::CasFailure);
}

#[test]
fn cas_success_has_update() {
    let net = common::translate_fixture("cas.json");

    let out = common::output_arcs(&net, "main_s1_branch_true");
    assert!(!out.is_empty());
    let update = common::output_update_by_name(&net, "main_s1_branch_true", &out[0].0)
        .expect("CAS success should have update");
    assert!(update.contains_key("flag"));
}

#[test]
fn cas_failure_no_update() {
    let net = common::translate_fixture("cas.json");

    let out = common::output_arcs(&net, "main_s1_branch_false");
    assert!(!out.is_empty());
    assert!(common::output_update_by_name(&net, "main_s1_branch_false", &out[0].0).is_none());
}
