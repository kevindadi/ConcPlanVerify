//! Value observation: atomic call-site events and mutex-inner events, and the
//! monitor's `var_eq` decision over them. Unknown stays unknown; a printed text
//! or a wrong value is never accepted.

use concir::instrument::wrap;
use concir::monitor::{monitor_with_program, parse_trace};
use serde_json::json;

const ATOMIC_SRC: &str = r#"
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;
fn w1(c: Arc<AtomicI32>) {
    let observed = c.load(Ordering::SeqCst);
    c.store(observed + 1, Ordering::SeqCst);
}
fn main() {
    let c = Arc::new(AtomicI32::new(0));
    let c1 = Arc::clone(&c);
    let h = thread::spawn(move || w1(c1));
    h.join().unwrap();
    println!("DONE done=1");
}
"#;

fn atomic_contract() -> serde_json::Value {
    json!({"properties": [
        {"kind": "always_reachable", "id": "c2",
         "goal": {"kind": "var_eq", "resource": "main::c", "value": 2}, "req": ["R3"]}
    ]})
}

fn safety_cmp_contract(op: &str, value: i64) -> serde_json::Value {
    json!({"properties": [
        {"kind": "safety", "id": "counter-bound",
         "invariant": {"kind": "var_cmp", "resource": "main::c", "op": op, "value": value},
         "req": ["R2"]}
    ]})
}

fn trace_values(vals: &[i64]) -> Vec<concir::monitor::TraceEvent> {
    let text = vals.iter().map(|v| format!(
        "{{\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":{v}}}")).collect::<Vec<_>>().join("\n");
    parse_trace(&text)
}

fn atomic_program() -> serde_json::Value {
    json!({"modules": [{"name": "main",
        "resources": [{"name": "c", "kind": "var", "type": "Atomic"}]}]})
}

fn guarded_contract() -> serde_json::Value {
    json!({"properties": [
        {"kind": "reachability", "id": "ready",
         "goal": {"kind": "var_eq", "resource": "main::ready", "value": true}, "req": ["R3"]}
    ]})
}

fn guarded_program() -> serde_json::Value {
    json!({"modules": [{"name": "main",
        "resources": [{"name": "ready", "kind": "var", "type": "Var"},
                      {"name": "m", "kind": "sync", "type": "Mutex"}],
        "protection": [{"var": "ready", "lock": "m"}]}]})
}

fn overrides(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
    pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

#[test]
fn atomic_receiver_emits_a_value_event() {
    let w = wrap(ATOMIC_SRC).expect("wrap");
    assert!(w.annotated.contains("cir_trace::record_value"),
            "atomic call site must record the real stored value");
    assert!(w.resources.iter().any(|r| r.kind == "Atomic" && r.display.as_deref() == Some("c")));
}

#[test]
fn var_eq_true_when_the_value_is_observed() {
    let traces = vec![parse_trace("{\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":2}")];
    let rep = monitor_with_program(&atomic_contract(), &["c".to_string()],
                                   &overrides(&[("c", "main::c")]), &traces,
                                   Some(&atomic_program()));
    assert_eq!(rep.properties[0].status, "PASS_bounded");
}

#[test]
fn var_eq_wrong_value_is_not_observed_not_pass() {
    // value 1 while the goal is 2; a printed DONE is irrelevant
    let traces = vec![parse_trace("{\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":1}")];
    let rep = monitor_with_program(&atomic_contract(), &["c".to_string()],
                                   &overrides(&[("c", "main::c")]), &traces,
                                   Some(&atomic_program()));
    assert_eq!(rep.properties[0].status, "not_observed");
}

#[test]
fn var_eq_without_a_value_event_is_unsupported() {
    let traces = vec![parse_trace("{\"t\":\"t1\",\"op\":\"mutex_lock\",\"r\":\"c\"}")];
    let rep = monitor_with_program(&atomic_contract(), &["c".to_string()],
                                   &overrides(&[("c", "main::c")]), &traces,
                                   Some(&atomic_program()));
    assert_eq!(rep.properties[0].status, "unsupported");
}

#[test]
fn guarded_var_is_observed_through_its_lock() {
    let traces = vec![parse_trace("{\"t\":\"t1\",\"op\":\"value\",\"r\":\"m0\",\"value\":1}")];
    let rep = monitor_with_program(&guarded_contract(), &["m0".to_string()],
                                   &overrides(&[("m0", "main::m")]), &traces,
                                   Some(&guarded_program()));
    assert_eq!(rep.properties[0].status, "PASS_bounded");
}

#[test]
fn unrelated_value_does_not_satisfy_the_guarded_var() {
    let traces = vec![parse_trace("{\"t\":\"t1\",\"op\":\"value\",\"r\":\"other\",\"value\":1}")];
    let rep = monitor_with_program(&guarded_contract(), &["m0".to_string()],
                                   &overrides(&[("m0", "main::m")]), &traces,
                                   Some(&guarded_program()));
    assert_eq!(rep.properties[0].status, "unsupported");
}

#[test]
fn intermediate_value_does_not_satisfy_a_higher_goal() {
    // c reached 1 but never 2 on this trace: not observed, not a pass
    let traces = vec![parse_trace(
        "{\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":0}\n\
         {\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":1}")];
    let rep = monitor_with_program(&atomic_contract(), &["c".to_string()],
                                   &overrides(&[("c", "main::c")]), &traces,
                                   Some(&atomic_program()));
    assert_eq!(rep.properties[0].status, "not_observed");
}

// ── same-state witness for compound predicates ──────────────────────────────

fn compound_contract(goal: serde_json::Value) -> serde_json::Value {
    json!({"properties": [{"kind": "reachability", "id": "compound",
                           "goal": goal, "req": ["R3"]}]})
}

#[test]
fn reachable_c2_and_c1_never_holds_in_one_state() {
    // history has both 2 and 1, but no single state has both current values
    let goal = json!({"kind": "and", "predicates": [
        {"kind": "var_eq", "resource": "main::c", "value": 2},
        {"kind": "var_eq", "resource": "main::c", "value": 1}]});
    let traces = vec![parse_trace(
        "{\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":2}\n\
         {\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":1}")];
    let rep = monitor_with_program(&compound_contract(goal), &["c".to_string()],
                                   &overrides(&[("c", "main::c")]), &traces,
                                   Some(&atomic_program()));
    assert_ne!(rep.properties[0].status, "PASS_bounded");
}

#[test]
fn reachable_c2_and_w1_completed_needs_a_common_state() {
    let goal = json!({"kind": "and", "predicates": [
        {"kind": "var_eq", "resource": "main::c", "value": 2},
        {"kind": "function_completed", "function": "main::w1"}]});
    // c hits 2, then drops to 1, only then w1 completes: no common state
    let traces = vec![parse_trace(
        "{\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":2}\n\
         {\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":1}\n\
         {\"t\":\"t1\",\"op\":\"complete\",\"r\":\"w1\"}")];
    let rep = monitor_with_program(&compound_contract(goal.clone()), &["c".to_string(), "w1".to_string()],
                                   &overrides(&[("c", "main::c"), ("w1", "main::w1")]), &traces,
                                   Some(&atomic_program()));
    assert_ne!(rep.properties[0].status, "PASS_bounded");
    // positive: c == 2 and w1 complete in the SAME state
    let pos = vec![parse_trace(
        "{\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":2}\n\
         {\"t\":\"t1\",\"op\":\"complete\",\"r\":\"w1\"}")];
    let rep2 = monitor_with_program(&compound_contract(goal), &["c".to_string(), "w1".to_string()],
                                    &overrides(&[("c", "main::c"), ("w1", "main::w1")]), &pos,
                                    Some(&atomic_program()));
    assert_eq!(rep2.properties[0].status, "PASS_bounded");
}

// ── type / overflow / negation ──────────────────────────────────────────────

#[test]
fn bool_goal_value_is_read_as_one_zero() {
    let traces = vec![parse_trace("{\"t\":\"t1\",\"op\":\"value\",\"r\":\"m0\",\"value\":1}")];
    let rep = monitor_with_program(&guarded_contract(), &["m0".to_string()],
                                   &overrides(&[("m0", "main::m")]), &traces,
                                   Some(&guarded_program()));
    assert_eq!(rep.properties[0].status, "PASS_bounded");
}

#[test]
fn large_goal_value_is_not_satisfied_by_a_smaller_observation() {
    let contract = json!({"properties": [{"kind": "reachability", "id": "big",
        "goal": {"kind": "var_eq", "resource": "main::c", "value": 4294967296u64}, "req": ["R3"]}]});
    let traces = vec![parse_trace("{\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":4294967295}")];
    let rep = monitor_with_program(&contract, &["c".to_string()],
                                   &overrides(&[("c", "main::c")]), &traces,
                                   Some(&atomic_program()));
    assert_eq!(rep.properties[0].status, "not_observed");
}

#[test]
fn contradictory_negation_does_not_pass() {
    // c == 2 AND NOT(c == 2) can never hold in one state
    let goal = json!({"kind": "and", "predicates": [
        {"kind": "var_eq", "resource": "main::c", "value": 2},
        {"kind": "not", "predicate": {"kind": "var_eq", "resource": "main::c", "value": 2}}]});
    let traces = vec![parse_trace("{\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":2}")];
    let rep = monitor_with_program(&compound_contract(goal), &["c".to_string()],
                                   &overrides(&[("c", "main::c")]), &traces,
                                   Some(&atomic_program()));
    assert_ne!(rep.properties[0].status, "PASS_bounded");
}

#[test]
fn var_cmp_and_var_ref_are_unsupported() {
    for kind in ["var_cmp", "var_ref"] {
        let goal = json!({"kind": kind, "resource": "main::c", "value": 2});
        let traces = vec![parse_trace("{\"t\":\"t1\",\"op\":\"value\",\"r\":\"c\",\"value\":2}")];
        let rep = monitor_with_program(&compound_contract(goal), &["c".to_string()],
                                       &overrides(&[("c", "main::c")]), &traces,
                                       Some(&atomic_program()));
        assert_eq!(rep.properties[0].status, "unsupported", "{kind} must stay unsupported");
    }
}

const NESTED_USE_SRC: &str = r#"
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::thread;

fn main() {
    let c = Arc::new(AtomicUsize::new(0));
    c.store(1, Ordering::SeqCst);
    let _ = c.load(Ordering::SeqCst);
    let h = thread::spawn(move || { let _ = c.load(Ordering::SeqCst); });
    h.join().unwrap();
}
"#;

const TYPED_MUTEX_SRC: &str = r#"
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let b: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || {
        let _g = a1.lock().unwrap();
        let _h = b1.lock().unwrap();
    });
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = thread::spawn(move || {
        let _g = b2.lock().unwrap();
        let _h = a2.lock().unwrap();
    });
    h1.join().unwrap();
    h2.join().unwrap();
}
"#;

const STRUCT_MUTEX_SRC: &str = r#"
use std::sync::{Arc, Mutex};
use std::thread;

struct State { c: i32 }

fn main() {
    let m: Arc<Mutex<State>> = Arc::new(Mutex::new(State { c: 0 }));
    let m2 = Arc::clone(&m);
    let h = thread::spawn(move || { let mut g = m2.lock().unwrap(); g.c += 1; });
    h.join().unwrap();
    let _ = m.lock().unwrap().c;
}
"#;

const DOUBLE_SPAWN_SRC: &str = r#"
use std::thread;

fn worker() {}

fn main() {
    let a = thread::spawn(move || worker());
    let b = thread::spawn(move || worker());
    a.join().unwrap();
    b.join().unwrap();
}
"#;

#[test]
fn same_worker_spawned_twice_keeps_distinct_creation_sites() {
    let w = wrap(DOUBLE_SPAWN_SRC).expect("wrap");
    let spawns: Vec<&concir::instrument::Resource> =
        w.resources.iter().filter(|r| r.kind == "Spawn").collect();
    assert_eq!(spawns.len(), 2, "two spawn sites must yield two resources");
    let names: std::collections::BTreeSet<&str> =
        spawns.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names.len(), 2, "creation sites must stay distinct, not order-guessed");
    assert!(spawns.iter().all(|r| r.entry.as_deref() == Some("worker")));
}

#[test]
fn struct_mutex_fields_are_observed() {
    let w = wrap(STRUCT_MUTEX_SRC).expect("wrap");
    assert!(w.annotated.contains("new_observed"),
            "a Mutex<Struct> constructor must register a field observer");
    assert!(w.annotated.contains("__cir_obs_State"),
            "an observer fn must be generated for the struct");
    assert!(w.annotated.contains("record_value(&format!(\"{}::c\", r)"),
            "the observer must record the real field value");
}

#[test]
fn typed_let_bindings_keep_distinct_names() {
    // `let a: Arc<Mutex<()>>` and `let b: Arc<Mutex<()>>` are two distinct
    // objects; the typed pattern must not collapse both to `res_mutex0`.
    let w = wrap(TYPED_MUTEX_SRC).expect("wrap");
    let displays: Vec<String> = w.resources.iter().filter(|r| r.kind == "Mutex")
        .filter_map(|r| r.display.clone()).collect();
    assert!(displays.iter().any(|d| d.starts_with("a_mutex")), "got {displays:?}");
    assert!(displays.iter().any(|d| d.starts_with("b_mutex")), "got {displays:?}");
    assert!(!displays.iter().any(|d| d.starts_with("res_mutex")),
            "typed bindings must not collapse to res_mutex0: {displays:?}");
}

#[test]
fn nested_use_group_is_not_corrupted() {
    let w = wrap(NESTED_USE_SRC).expect("wrap");
    // The nested `atomic::{..}` item and the sibling `Arc` must survive intact;
    // the old bug produced `use std::sync::{atomic::{AtomicUsize, Ordering};`.
    assert!(w.annotated.contains("atomic::{AtomicUsize, Ordering}"),
            "nested atomic import must be preserved");
    assert!(!w.annotated.contains("use std::sync::{atomic::{AtomicUsize, Ordering};"),
            "corrupted unbalanced use must not be produced");
}

// ── var_cmp comparison predicates (safety, ForAll over observed states) ──────

fn cmp_status(contract: &serde_json::Value, vals: &[i64]) -> String {
    let rep = monitor_with_program(contract, &["c".to_string()],
                                   &overrides(&[("c", "main::c")]), &[trace_values(vals)],
                                   Some(&atomic_program()));
    rep.properties[0].status.clone()
}

#[test]
fn var_cmp_all_observed_within_bound_pass() {
    assert_eq!(cmp_status(&safety_cmp_contract("<=", 2), &[0, 1, 2]), "PASS_bounded");
}

#[test]
fn var_cmp_observed_violation_fails() {
    assert_eq!(cmp_status(&safety_cmp_contract("<=", 2), &[0, 1, 3]), "FAIL");
}

#[test]
fn var_cmp_boundary_values() {
    assert_eq!(cmp_status(&safety_cmp_contract("<=", 2), &[2]), "PASS_bounded");
    assert_eq!(cmp_status(&safety_cmp_contract("<", 2), &[2]), "FAIL");
    assert_eq!(cmp_status(&safety_cmp_contract(">=", 2), &[1]), "FAIL");
    assert_eq!(cmp_status(&safety_cmp_contract(">", 2), &[2]), "FAIL");
    assert_eq!(cmp_status(&safety_cmp_contract("==", 2), &[2]), "PASS_bounded");
    assert_eq!(cmp_status(&safety_cmp_contract("!=", 2), &[2]), "FAIL");
}

#[test]
fn var_cmp_mid_run_violation_then_recovery_still_fails() {
    assert_eq!(cmp_status(&safety_cmp_contract("<=", 2), &[0, 3, 2]), "FAIL");
}

#[test]
fn var_cmp_without_a_value_event_is_unsupported_not_pass() {
    let rep = monitor_with_program(&safety_cmp_contract("<=", 2), &["c".to_string()],
                                   &overrides(&[("c", "main::c")]),
                                   &[parse_trace("{\"t\":\"t1\",\"op\":\"mutex_lock\",\"r\":\"c\"}")],
                                   Some(&atomic_program()));
    assert_eq!(rep.properties[0].status, "unsupported");
}

#[test]
fn var_cmp_unsupported_operator_is_unsupported() {
    let rep = monitor_with_program(&safety_cmp_contract("~=", 2), &["c".to_string()],
                                   &overrides(&[("c", "main::c")]), &[trace_values(&[1])],
                                   Some(&atomic_program()));
    assert_eq!(rep.properties[0].status, "unsupported");
}

#[test]
fn var_cmp_observes_a_struct_field_through_its_lock() {
    let contract = json!({"properties": [
        {"kind": "safety", "id": "counter-bound",
         "invariant": {"kind": "var_cmp", "resource": "main::c", "op": "<=", "value": 2},
         "req": ["R2"]}]});
    let program = json!({"modules": [{"name": "main",
        "resources": [{"name": "c", "kind": "var", "type": "Var"},
                      {"name": "m", "kind": "sync", "type": "Mutex"}],
        "protection": [{"var": "c", "lock": "m"}]}]});
    let traces = vec![parse_trace(
        "{\"t\":\"t1\",\"op\":\"value\",\"r\":\"m0::c\",\"value\":1}\n\
         {\"t\":\"t1\",\"op\":\"value\",\"r\":\"m0::c\",\"value\":3}")];
    let rep = monitor_with_program(&contract, &["m0".to_string()],
                                   &overrides(&[("m0", "main::m")]), &traces, Some(&program));
    assert_eq!(rep.properties[0].status, "FAIL");
}


