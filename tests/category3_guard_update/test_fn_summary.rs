use crate::common;
use serde_json::json;
use unipn::model::TransitionKind;
use unipn::{Expr, Val};

/// Call to a body-less ("nobody") callee is an atomic pass-through: the callee
/// is a codegen placeholder with no control flow, so the caller's control flow
/// is not routed through a skeleton.
#[test]
fn fn_summary_call_transition() {
    let net = common::translate_fixture("fn_summary.json");

    let k = common::transition_kind(&net, "main_s1_call").unwrap();
    assert_eq!(k, TransitionKind::Call);
}

#[test]
fn fn_summary_writes_unknown() {
    let net = common::translate_fixture("fn_summary.json");

    let out = common::output_arcs(&net, "main_s1_call");
    assert!(!out.is_empty());
    let update = common::output_update_by_name(&net, "main_s1_call", &out[0].0)
        .expect("body-less call should apply effects writes");
    assert_eq!(update.get("result"), Some(&Expr::Lit(Val::Unknown)));
}

#[test]
fn bodyless_call_does_not_model_skeleton() {
    let net = common::translate_fixture("fn_summary.json");

    // The placeholder callee must not enter the net (no skeleton, no places).
    assert!(!common::has_place(&net, "validate.s_first"));
    assert!(!common::has_place(&net, "validate.ret"));
    assert!(!common::has_transition(&net, "main_s1_call_ret"));
    assert!(!common::has_transition(&net, "validate_body"));
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
    let net = common::translate_program(&program);

    // Entry transition enters the callee and parks the continuation.
    assert!(common::has_transition(&net, "w_s1_call"));
    assert!(common::has_place(&net, "helper.s_first"));
    assert!(common::has_place(&net, "w.s1_callwait"));

    // Return handoff consumes callee return + parked continuation (Join).
    let k = common::transition_kind(&net, "w_s1_call_ret").unwrap();
    assert_eq!(k, TransitionKind::Join);
    assert!(common::has_place(&net, "helper.ret"));

    // The callee body (lock/drop) is in the model.
    assert!(common::has_transition(&net, "helper_s1_lock"));
    assert!(common::has_transition(&net, "helper_s2_unlock"));
}
