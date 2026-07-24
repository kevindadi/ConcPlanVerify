use std::path::Path;

use cir::ast::Program;

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
    let report = cir::validate::validate(&program);

    assert!(report.valid, "canonical CIR is invalid: {:?}", report.diagnostics);
    assert!(report
        .diagnostics
        .iter()
        .all(|diagnostic| diagnostic.code != "E601"));
}

#[test]
fn canonical_schema_translates_and_goals_have_no_warnings() {
    let program = load_canonical_fixture();
    let net = cir2cvn::translate(&program).expect("canonical CIR should translate");
    assert!(net.place_count() > 0);
    assert!(net.transition_count() > 0);

    let (goals, warnings) = cir2cvn::translate_goals(&program);
    assert_eq!(goals.len(), 2);
    assert!(warnings.is_empty(), "unexpected goal warnings: {warnings:?}");
}
