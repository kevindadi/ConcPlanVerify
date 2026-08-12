use crate::common;
use cir2cvn::{VerificationConfig, verify_program};
use serde_json::json;
use unipn::analysis::{AnalysisConfig, SearchStrategy, explore};

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
    let net = common::translate_program(&dataflow_program());

    // Modeled param + return vars are in the store.
    let vars = common::initial_vars(&net);
    assert!(
        vars.contains_key("p_worker_n"),
        "param var missing: {vars:?}"
    );
    assert!(
        vars.contains_key("r_worker_out"),
        "return var missing: {vars:?}"
    );
    // Projection: the unmodeled param stays out of the net entirely.
    assert!(
        !vars.contains_key("p_worker_label"),
        "unmodeled param must not materialize, got: {vars:?}"
    );

    // The call transition binds p_worker_n = 5.
    let call_update = common::output_update_by_name(&net, "main_s1_call", "worker.s_first")
        .expect("call should carry a param binding update");
    let bindings: Vec<String> = call_update
        .iter()
        .map(|(k, v)| format!("{k}={v:?}"))
        .collect();
    assert!(
        bindings.iter().any(|b| b.starts_with("p_worker_n")),
        "call should bind param, got: {bindings:?}"
    );

    // The worker branch guard reads the namespaced param variable.
    let guard = common::input_guard_by_name(&net, "worker_s1_branch_true", "worker.s1")
        .expect("worker branch guard");
    let guard_json = serde_json::to_string(&guard).unwrap();
    assert!(
        guard_json.contains("p_worker_n"),
        "guard should reference p_worker_n, got: {guard_json}"
    );

    // The call_ret handoff captures the modeled return into the caller Var.
    let capture = common::output_update_by_name(&net, "main_s1_call_ret", "main.s2")
        .expect("call_ret should carry a capture update");
    let capture_keys: Vec<&String> = capture.keys().collect();
    assert!(
        capture_keys.contains(&&"result".to_string()),
        "call_ret should capture into result, got: {capture_keys:?}"
    );
}

#[test]
fn dataflow_reaches_goal_through_called_parameters() {
    // A goal on the captured return (result == 42) must be reachable: the
    // value flows call-arg → param → guard → return → captured Var.
    let mut program = dataflow_program();
    program.goals = vec![
        serde_json::from_value(json!({
            "id": "g_result",
            "marking": {},
            "variables": {"result": 42}
        }))
        .expect("goal must parse"),
    ];

    let result = verify_program(&program, &VerificationConfig::default());
    assert!(
        result.unmet_goals.is_empty(),
        "result=42 must be reachable, unmet: {:?}",
        result.unmet_goals
    );
}

/// An infinite-looking counter loop (`s1 → count=count+1 → s1`, no exit guard)
/// with a BOUNDED Int counter terminates: the increment leaving the domain
/// disables the transition, so exploration completes instead of exhausting the
/// state budget.
#[test]
fn bounded_int_counter_loop_terminates() {
    let program: concir::ast::Program = serde_json::from_value(json!({
        "program": "bounded_loop",
        "resources": [
            {"name": "count", "kind": "var", "type": "Var", "base": {"Int": [0, 4]}, "init": 0}
        ],
        "protection": [],
        "functions": [
            {
                "name": "main", "kind": "normal",
                "body": [
                    {"sid": "s1", "op": ["spawn", "worker"], "transfer": ["next", "s2"]},
                    {"sid": "s2", "op": ["join", "worker"], "transfer": ["next", "s3"]},
                    {"sid": "s3", "op": "return", "transfer": "return"}
                ]
            },
            {
                "name": "worker", "kind": "closure",
                "body": [
                    {"sid": "s1", "op": ["res_op", "count", "write", "count + 1"], "transfer": ["next", "s2"]},
                    {"sid": "s2", "op": "nop", "transfer": ["next", "s1"]}
                ]
            }
        ],
        "entry": "main"
    }))
    .expect("test CIR must parse");

    let net = common::translate_program(&program);

    // The variable carries its declared domain.
    assert_eq!(
        net.initial.extra.domains.get("count").copied(),
        Some((0, 4))
    );

    // Exploration terminates well within a tiny state budget (no unbounded
    // growth): count stays in [0,4] and the loop stops at the bound.
    let config = AnalysisConfig {
        strategy: SearchStrategy::Bfs,
        max_states: 100_000,
        ..AnalysisConfig::default()
    };
    let result = explore(&net.net, net.initial.clone(), &config);
    assert!(
        result.state_count() < 100,
        "bounded counter loop should terminate in a tiny state space, explored {} states",
        result.state_count()
    );
}
