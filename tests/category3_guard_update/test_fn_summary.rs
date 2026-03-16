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

    let tid = TransitionId::new("main_s1_call");
    let out = net.output_arcs(&tid);
    assert!(!out.is_empty());
    let update = out[0].update.as_ref().expect("Call should have update for writes");
    assert_eq!(update.get("result"), Some(&Expr::Lit(Val::Unknown)));
}
