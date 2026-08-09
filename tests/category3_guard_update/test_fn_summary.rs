use crate::common;
use cvn::model::{Expr, TransitionId, TransitionKind, Val};

#[test]
fn fn_summary_call_transition() {
    let net = common::translate_fixture("fn_summary.json");

    let tid = TransitionId::new("main_s1_call");
    let t = net.transition(&tid).unwrap();
    assert!(matches!(t.kind, TransitionKind::Call));
}

#[test]
fn fn_summary_writes_unknown() {
    let net = common::translate_fixture("fn_summary.json");

    // The callee `validate` is body-less; its effects transition carries the
    // writes (as Unknown), not the call transition itself.
    let tid = TransitionId::new("validate_body");
    let t = net.transition(&tid).expect("body-less callee skeleton");
    assert!(matches!(t.kind, TransitionKind::Sequential));

    let out = net.output_arcs(&tid);
    assert!(!out.is_empty());
    let update = out[0]
        .update
        .as_ref()
        .expect("body transition should have update for writes");
    assert_eq!(update.get("result"), Some(&Expr::Lit(Val::Unknown)));
}

#[test]
fn call_expands_through_callee_skeleton() {
    let net = common::translate_fixture("fn_summary.json");

    // Call enters the callee entry place and parks the caller continuation.
    let call_tid = TransitionId::new("main_s1_call");
    assert!(net.transition(&call_tid).is_some());
    assert!(common::has_place(&net, "cp_validate_s_first"));
    assert!(common::has_place(&net, "cp_main_s1_callwait"));

    // Return handoff consumes callee return + parked continuation.
    let ret_tid = TransitionId::new("main_s1_call_ret");
    let ret = net.transition(&ret_tid).expect("call return handoff");
    assert!(matches!(ret.kind, TransitionKind::Join));
    assert!(common::has_place(&net, "cp_validate_ret"));
}
