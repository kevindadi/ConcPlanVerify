use std::path::{Path, PathBuf};
use std::process::Command;

use cir2cvn::{verify_program, VerificationConfig, VerificationStatus};

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn load_program(relative: &str) -> cir::ast::Program {
    let path = repo_path(relative);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&source)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

fn run_cli(mode: &str, relative: &str) -> (bool, serde_json::Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_cir2cvn"))
        .args([mode, repo_path(relative).to_str().unwrap()])
        .output()
        .unwrap_or_else(|e| panic!("failed to run cir2cvn: {e}"));
    let stdout = String::from_utf8(output.stdout).expect("CLI stdout must be UTF-8");
    let json = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("CLI stdout is not JSON: {e}\nstdout: {stdout}"));
    (output.status.success(), json)
}

#[test]
fn safe_fixture_is_verified_by_both_analysis_aliases() {
    for mode in ["--analyze", "--goals"] {
        let (success, output) = run_cli(mode, "tests/fixtures/sequential_chain.json");
        assert!(success, "{mode} should exit successfully: {output}");
        assert_eq!(output["status"], "verified_safe");
        assert_eq!(output["analysis_complete"], true);
        assert!(output["state_count"].as_u64().unwrap() > 0);
        assert!(output["bugs"].as_array().unwrap().is_empty());
    }
}

#[test]
fn buggy_fixture_is_reported_with_a_nonzero_exit() {
    let (success, output) = run_cli("--analyze", "tests/e2e/mutex_deadlock/buggy.json");
    assert!(!success, "unsafe verification must fail CI: {output}");
    assert_eq!(output["status"], "verified_unsafe");
    assert!(output["bugs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|bug| { bug["kind"]["Deadlock"].is_object() }));
}

#[test]
fn unmet_goal_is_distinguished_from_a_concurrency_bug() {
    let (success, output) = run_cli("--goals", "tests/fixtures/unmet_goal.json");
    assert!(!success, "unmet goals must fail CI: {output}");
    assert_eq!(output["status"], "goals_unmet");
    assert_eq!(output["declared_goal_count"], 1);
    assert_eq!(output["unmet_goals"].as_array().unwrap().len(), 1);
    assert!(output["bugs"].as_array().unwrap().is_empty());
}

#[test]
fn state_limit_produces_an_incomplete_result() {
    let program = load_program("tests/fixtures/sequential_chain.json");
    let result = verify_program(
        &program,
        &VerificationConfig {
            max_states: 1,
            ..VerificationConfig::default()
        },
    );

    assert_eq!(result.status, VerificationStatus::AnalysisIncomplete);
    assert!(!result.analysis_complete);
    assert!(result.analysis_error.is_some());
}

#[test]
fn invalid_cir_stops_before_translation() {
    let program: cir::ast::Program = serde_json::from_value(serde_json::json!({
        "program": "invalid",
        "resources": [],
        "protection": [],
        "functions": [{
            "name": "main",
            "kind": "normal",
            "body": [{
                "sid": "s1",
                "op": "return",
                "transfer": ["next", "missing"]
            }]
        }],
        "fn_summaries": [],
        "entry": "main"
    }))
    .expect("invalid fixture should still be valid JSON/CIR syntax");

    let result = verify_program(&program, &VerificationConfig::default());
    assert_eq!(result.status, VerificationStatus::InvalidModel);
    assert!(!result.validation.valid);
    assert!(result.translation_errors.is_empty());
}

#[test]
fn control_flow_and_async_fixtures_pass_the_unified_pipeline() {
    for (relative, expected_status) in [
        (
            "tests/fixtures/branch.json",
            VerificationStatus::VerifiedUnsafe,
        ),
        (
            "tests/fixtures/switch.json",
            VerificationStatus::VerifiedUnsafe,
        ),
        (
            "cir/examples/async_workers.json",
            VerificationStatus::VerifiedSafe,
        ),
    ] {
        let program = load_program(relative);
        let result = verify_program(&program, &VerificationConfig::default());
        assert_eq!(
            result.status, expected_status,
            "{relative}: validation={:?}, translation={:?}, analysis={:?}",
            result.validation, result.translation_errors, result.analysis_error
        );
        assert!(result.analysis_complete, "{relative}");
    }
}
