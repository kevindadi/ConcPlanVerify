use crate::common;
use cvn::model::{Expr, TransitionId, TransitionKind, Val};
use serde_json::json;

/// Call to a body-less ("nobody") callee is an atomic pass-through: the callee
/// is a codegen placeholder with no control flow, so the caller's control flow
/// is not routed through a skeleton.
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
    let update = out[0]
        .update
        .as_ref()
        .expect("body-less call should apply effects writes");
    assert_eq!(update.get("result"), Some(&Expr::Lit(Val::Unknown)));
}

#[test]
fn bodyless_call_does_not_model_skeleton() {
    let net = common::translate_fixture("fn_summary.json");

    // The placeholder callee must not enter the net (no skeleton, no places).
    assert!(!common::has_place(&net, "cp_validate_s_first"));
    assert!(!common::has_place(&net, "cp_validate_ret"));
    assert!(net.transition(&TransitionId::new("main_s1_call_ret")).is_none());
    assert!(net.transition(&TransitionId::new("validate_body")).is_none());
}

/// Call to a bodied callee expands into its skeleton: enter the callee entry,
/// park the caller continuation on a `_callwait` place, and hand back via a
/// Join that consumes the callee return. This is where cross-function lock
/// chains become visible to the analysis.
#[test]
fn call_expands_through_bodied_callee_skeleton() {
    let program: concir::ast::Program = serde_json::from_value(json!({
        "program": "bodied_call",
        "resources": [
            {"name": "m1", "kind": "sync", "type": "Mutex", "mode": "Sync"}
        ],
        "protection": [],
        "functions": [
            {
                "name": "main", "kind": "normal",
                "body": [
                    {"sid": "s1", "op": ["spawn", "w"], "transfer": ["next", "s2"]},
                    {"sid": "s2", "op": ["join", "w"], "transfer": ["next", "s3"]},
                    {"sid": "s3", "op": "return", "transfer": "return"}
                ]
            },
            {
                "name": "w", "kind": "closure",
                "body": [
                    {"sid": "s1", "op": ["call", "helper"], "transfer": ["next", "s2"]},
                    {"sid": "s2", "op": "return", "transfer": "return"}
                ]
            },
            {
                "name": "helper", "kind": "normal",
                "body": [
                    {"sid": "s1", "op": ["res_op", "m1", "lock"], "transfer": ["next", "s2"]},
                    {"sid": "s2", "op": ["res_op", "m1", "drop"], "transfer": ["next", "s3"]},
                    {"sid": "s3", "op": "return", "transfer": "return"}
                ]
            }
        ],
        "entry": "main"
    }))
    .expect("test ConcIR must parse");
    let net = cir2cvn::translate(&program).expect("translation should succeed");

    // Entry transition enters the callee and parks the continuation.
    assert!(net.transition(&TransitionId::new("w_s1_call")).is_some());
    assert!(common::has_place(&net, "cp_helper_s_first"));
    assert!(common::has_place(&net, "cp_w_s1_callwait"));

    // Return handoff consumes callee return + parked continuation (Join).
    let ret = net.transition(&TransitionId::new("w_s1_call_ret")).expect("call return handoff");
    assert!(matches!(ret.kind, TransitionKind::Join));
    assert!(common::has_place(&net, "cp_helper_ret"));

    // The callee body (lock/drop) is in the model.
    assert!(net.transition(&TransitionId::new("helper_s1_lock")).is_some());
    assert!(net.transition(&TransitionId::new("helper_s2_unlock")).is_some());
}
