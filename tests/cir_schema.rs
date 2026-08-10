use std::path::Path;

use concir::ast::Program;
use unipn::NetLike;

fn load_canonical_fixture() -> Program {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/canonical_schema.json");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&source)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

#[test]
fn canonical_schema_parses_and_validates() {
    let program = load_canonical_fixture();
    let report = concir::validate::validate(&program);

    assert!(report.valid, "canonical ConcIR is invalid: {:?}", report.diagnostics);
    assert!(report
        .diagnostics
        .iter()
        .all(|diagnostic| diagnostic.code != "E601"));
}

#[test]
fn canonical_schema_translates_and_goals_have_no_warnings() {
    let program = load_canonical_fixture();
    let net = cir2cvn::translate(&program).expect("canonical ConcIR should translate");
    assert!(net.num_places() > 0);
    assert!(net.num_transitions() > 0);

    let (goals, warnings) = cir2cvn::translate_goals(&program, &net);
    assert_eq!(goals.len(), 2);
    assert!(warnings.is_empty(), "unexpected goal warnings: {warnings:?}");
}

#[test]
fn legacy_unknown_fields_are_rejected() {
    let legacy = r#"
    {
      "program": "legacy",
      "resources": [
        {"name": "mtx", "kind": "sync", "type": "Mutex", "mode": "Sync"},
        {"name": "cv", "kind": "sync", "type": "Condvar", "mode": "Sync", "paired_with": "mtx"}
      ],
      "protection": [],
      "functions": [{
        "name": "main",
        "kind": "normal",
        "body": [{"sid": "s1", "op": "return", "transfer": "return"}]
      }],
      "entry": "main"
    }
    "#;

    assert!(serde_json::from_str::<Program>(legacy).is_err());
}

#[test]
fn operation_tuples_have_strict_shapes() {
    assert!(serde_json::from_str::<Program>(
        r#"{
          "program":"bad_op",
          "resources":[], "protection":[],
          "functions":[{"name":"main","kind":"normal","body":[
            {"sid":"s1","op":["spawn","worker","unexpected"],"transfer":"return"}
          ]}],
          "entry":"main"
        }"#
    )
    .is_err());

    assert!(serde_json::from_str::<Program>(
        r#"{
          "program":"bad_transfer",
          "resources":[], "protection":[],
          "functions":[{"name":"main","kind":"normal","body":[
            {"sid":"s1","op":"return","transfer":["next","s1","unexpected"]}
          ]}],
          "entry":"main"
        }"#
    )
    .is_err());
}

#[test]
fn resource_actions_have_canonical_names_and_arity() {
    let source = r#"
    {
      "program": "bad_actions",
      "resources": [
        {"name": "cv", "kind": "sync", "type": "Condvar", "mode": "Sync"}
      ],
      "protection": [],
      "functions": [{
        "name": "main",
        "kind": "normal",
        "body": [
          {"sid": "s1", "op": ["res_op", "cv", "notify_one"], "transfer": "return"}
        ]
      }],
      "entry": "main"
    }
    "#;

    let program: Program = serde_json::from_str(source).expect("JSON shape should parse");
    let report = concir::validate::validate(&program);

    assert!(!report.valid);
    assert!(report.diagnostics.iter().any(|d| d.code == "E310"));
}

#[test]
fn resource_action_arity_is_strict() {
    let source = r#"
    {
      "program": "bad_arity",
      "resources": [
        {"name": "cv", "kind": "sync", "type": "Condvar", "mode": "Sync"}
      ],
      "protection": [],
      "functions": [{
        "name": "main",
        "kind": "normal",
        "body": [
          {"sid": "s1", "op": ["res_op", "cv", "notify", "unexpected"], "transfer": ["next", "s2"]},
          {"sid": "s2", "op": ["res_op", "cv", "wait"], "transfer": ["next", "s3"]},
          {"sid": "s3", "op": "return", "transfer": "return"}
        ]
      }],
      "entry": "main"
    }
    "#;

    let program: Program = serde_json::from_str(source).expect("JSON shape should parse");
    let report = concir::validate::validate(&program);

    assert!(!report.valid);
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "E311")
            .count(),
        2
    );
}
