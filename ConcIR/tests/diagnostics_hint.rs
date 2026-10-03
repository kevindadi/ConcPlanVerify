//! `holds_all` / `never_holds_all` preservation failures carry a machine-generated
//! repair hint (no task names; function/resource names only).

use concir::ast::Program;
use concir::explore::contract::ContractSpec;
use concir::explore::{verify_program, EngineKind};
use concir::sem::outcome::Outcome;

const RELEASE_EARLY: &str = r#"{
  "program": "release_early",
  "version": "3.5.0",
  "entry": "main::main",
  "modules": [{
    "name": "main",
    "resources": [
      {"name": "a", "kind": "sync", "type": "Mutex", "mode": "Sync"},
      {"name": "b", "kind": "sync", "type": "Mutex", "mode": "Sync"}
    ],
    "protection": [],
    "functions": [
      {"name": "main", "kind": "normal", "body": [
        {"sid": "s1", "kind": "scope", "funcs": ["main::w"]},
        {"sid": "s2", "kind": "return"}
      ]},
      {"name": "w", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::a"},
        {"sid": "s2", "kind": "mutex_unlock", "resource": "main::a"},
        {"sid": "s3", "kind": "mutex_lock", "resource": "main::b"},
        {"sid": "s4", "kind": "mutex_unlock", "resource": "main::b"},
        {"sid": "s5", "kind": "return"}
      ]}
    ]
  }]
}"#;

const CONTRACT: &str = r#"{
  "name": "hint",
  "properties": [{"kind": "deadlock_free", "id": "no-deadlock"}],
  "preserved": [
    {"kind": "reachable", "description": "w holds a and b at once",
     "goal": {"kind": "holds_all", "function": "main::w",
              "resources": ["main::a", "main::b"]}}
  ],
  "allowed_scope": {"allow_lock_reorder": true}
}"#;

#[test]
fn holds_all_failure_has_template_hint() {
    let program: Program = serde_json::from_str(RELEASE_EARLY).unwrap();
    let spec: ContractSpec = serde_json::from_str(CONTRACT).unwrap();
    let report = verify_program(&program, &spec, EngineKind::Petri);
    assert_eq!(report.outcome, Outcome::Fail);
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.property.contains("holds a and b"))
        .expect("holds_all diagnostic present");
    let hints = diag.repair_hints.join(" | ");
    assert!(hints.contains("holds all of"), "hint text: {hints}");
    assert!(hints.contains("`w`"), "hint names function: {hints}");
    assert!(hints.contains("[a, b]"), "hint names resources: {hints}");
    assert!(hints.contains("releasing early"), "hint suggests the fix: {hints}");
}

// A truncated exploration is UNKNOWN, not FAIL, and carries the CIR modeling
// facts that explain the limit (here: a worker started at two sites).
const DUPLICATE_SPAWN: &str = r#"{
  "program": "dup_spawn",
  "version": "3.5.0",
  "entry": "main::main",
  "modules": [{
    "name": "main",
    "resources": [{"name": "m", "kind": "sync", "type": "Mutex", "mode": "Sync"}],
    "protection": [],
    "functions": [
      {"name": "main", "kind": "normal", "body": [
        {"sid": "s1", "kind": "spawn", "func": "main::w", "handle": "h1"},
        {"sid": "s2", "kind": "spawn", "func": "main::w", "handle": "h2"},
        {"sid": "s3", "kind": "join", "handle": "h1"},
        {"sid": "s4", "kind": "join", "handle": "h2"},
        {"sid": "s5", "kind": "return"}
      ]},
      {"name": "w", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s3", "kind": "return"}
      ]}
    ]
  }]
}"#;

const TINY_BOUNDS: &str = r#"{
  "name": "tiny",
  "properties": [{"kind": "deadlock_free", "id": "no-deadlock"}],
  "bounds": {"max_states": 2, "max_depth": 4, "max_threads": 8,
             "max_frames_per_thread": 8, "max_boundary_events": 8}
}"#;

#[test]
fn truncated_exploration_reports_modeling_facts_not_fail() {
    let program: Program = serde_json::from_str(DUPLICATE_SPAWN).unwrap();
    let spec: ContractSpec = serde_json::from_str(TINY_BOUNDS).unwrap();
    let report = verify_program(&program, &spec, EngineKind::Petri);
    assert_eq!(report.outcome, Outcome::Unknown);
    assert!(!report.complete);
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.property == "exploration")
        .expect("truncation diagnostic present");
    let facts = diag.proven_facts.join(" | ");
    assert!(facts.contains("w started 2 times"), "facts: {facts}");
    assert!(facts.contains("2 spawn activation"), "facts: {facts}");
    let hints = diag.repair_hints.join(" | ");
    assert!(hints.contains("reduce unnecessary concurrent activations"),
            "hints: {hints}");
}
