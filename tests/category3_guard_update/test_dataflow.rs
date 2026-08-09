use crate::common;
use cir2cvn::{verify_program, VerificationConfig};
use cvn::model::TransitionId;
use serde_json::json;

/// Bounded typed data flow with projection:
/// - modeled params are materialized as namespaced CVN variables
///   (`p_worker_n`), bound by the call transition, readable in guards.
/// - a modeled return (`r_worker_out`) is written by the return op and
///   captured into a caller Var via the call_ret handoff.
fn dataflow_program() -> concir::ast::Program {
    serde_json::from_value(json!({
        "program": "dataflow",
        "resources": [
            {"name": "mtx", "kind": "sync", "type": "Mutex", "mode": "Sync"},
            {"name": "result", "kind": "var", "type": "Var", "base": "Int", "init": 0}
        ],
        "protection": [],
        "functions": [
            {
                "name": "main", "kind": "normal",
                "body": [
                    {"sid": "s1", "op": ["call", "worker", "result", "5"], "transfer": ["next", "s2"]},
                    {"sid": "s2", "op": ["res_op", "mtx", "lock"], "transfer": ["next", "s3"]},
                    {"sid": "s3", "op": ["res_op", "mtx", "drop"], "transfer": ["next", "s4"]},
                    {"sid": "s4", "op": "return", "transfer": "return"}
                ]
            },
            {
                "name": "worker", "kind": "normal",
                "params": [
                    {"name": "n", "type": "Int", "modeled": true},
                    {"name": "label", "type": "String", "modeled": false}
                ],
                "returns": {"name": "out", "type": "Int", "modeled": true},
                "body": [
                    {"sid": "s1", "op": "nop", "transfer": ["branch", "n > 3", "s2", "s3"]},
                    {"sid": "s2", "op": ["return", "42"], "transfer": "return"},
                    {"sid": "s3", "op": ["return", "0"], "transfer": "return"}
                ]
            }
        ],
        "entry": "main"
    }))
    .expect("test CIR must parse")
}

#[test]
fn modeled_params_materialize_and_guard_resolves() {
    let net = cir2cvn::translate(&dataflow_program()).expect("translation should succeed");

    // Modeled param + return vars are in the store.
    let vars = net.initial_vars();
    assert!(vars.contains_key("p_worker_n"), "param var missing: {vars:?}");
    assert!(vars.contains_key("r_worker_out"), "return var missing: {vars:?}");
    // Projection: the unmodeled param stays out of the net entirely.
    assert!(
        !vars.contains_key("p_worker_label"),
        "unmodeled param must not materialize, got: {vars:?}"
    );

    // The call transition binds p_worker_n = 5.
    let call = TransitionId::new("main_s1_call");
    let call_out = net.output_arcs(&call);
    let bindings: String = call_out
        .iter()
        .filter_map(|a| a.update.as_ref())
        .flat_map(|u| u.iter().map(|(k, v)| format!("{k}={v:?}")))
        .collect::<Vec<_>>()
        .join(",");
    assert!(bindings.contains("p_worker_n"), "call should bind param, got: {bindings}");

    // The worker branch guard reads the namespaced param variable.
    let true_tid = TransitionId::new("worker_s1_branch_true");
    let guard_json = net
        .input_arcs(&true_tid)
        .first()
        .map(|a| serde_json::to_string(&a.guard).unwrap())
        .unwrap_or_default();
    assert!(
        guard_json.contains("p_worker_n"),
        "guard should reference p_worker_n, got: {guard_json}"
    );

    // The call_ret handoff captures the modeled return into the caller Var.
    let ret = TransitionId::new("main_s1_call_ret");
    let ret_out = net.output_arcs(&ret);
    let capture: String = ret_out
        .iter()
        .filter_map(|a| a.update.as_ref())
        .flat_map(|u| u.iter().map(|(k, v)| format!("{k}={v:?}")))
        .collect::<Vec<_>>()
        .join(",");
    assert!(
        capture.contains("result") && capture.contains("r_worker_out"),
        "call_ret should capture r_worker_out into result, got: {capture}"
    );
}

#[test]
fn dataflow_reaches_goal_through_called_parameters() {
    // A goal on the captured return (result == 42) must be reachable: the
    // value flows call-arg → param → guard → return → captured Var.
    let mut program = dataflow_program();
    program.goals = vec![serde_json::from_value(json!({
        "id": "g_result",
        "marking": {},
        "variables": {"result": 42}
    }))
    .expect("goal must parse")];

    let result = verify_program(&program, &VerificationConfig::default());
    assert!(
        result.unmet_goals.is_empty(),
        "result=42 must be reachable, unmet: {:?}",
        result.unmet_goals
    );
}
