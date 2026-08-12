use std::path::{Path, PathBuf};
use std::process::Command;

use cir2cvn::{VerificationConfig, VerificationStatus, verify_program};

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn load_program(relative: &str) -> concir::ast::Program {
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

fn run_cli_input(mode: &str, input: &str) -> (bool, serde_json::Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_cir2cvn"))
        .args([mode, "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .expect("CLI stdin should be piped")
                .write_all(input.as_bytes())?;
            let output = child.wait_with_output()?;
            Ok(output)
        })
        .unwrap_or_else(|e| panic!("failed to run cir2cvn with stdin: {e}"));
    let stdout = String::from_utf8(output.stdout).expect("CLI stdout must be UTF-8");
    let json = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("CLI stdout is not JSON: {e}\nstdout: {stdout}"));
    (output.status.success(), json)
}

#[test]
fn malformed_json_has_the_same_structured_error_shape_for_all_modes() {
    for mode in ["--validate", "--analyze", "--goals"] {
        let (success, output) = run_cli_input(mode, "not-json");
        assert!(!success, "malformed JSON must fail: {mode}");
        assert_eq!(output["status"], "invalid_json");
        assert_eq!(output["valid"], false);
        assert!(output["error"].as_str().is_some());
        assert_eq!(output["diagnostics"][0]["code"], "E000");
    }
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
    assert!(
        output["bugs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|bug| { bug["kind"]["Deadlock"].is_object() })
    );
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
    let program: concir::ast::Program = serde_json::from_value(serde_json::json!({
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
        "entry": "main"
    }))
    .expect("invalid fixture should still be valid JSON/ConcIR syntax");

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

#[test]
fn modular_deadlock_reports_source_modules() {
    let lock_body = |mutexes: (&str, &str)| {
        let (first, second) = mutexes;
        vec![
            serde_json::json!({"sid": "s1", "op": ["res_op", first, "lock"], "transfer": ["next", "s2"]}),
            serde_json::json!({"sid": "s2", "op": ["res_op", second, "lock"], "transfer": ["next", "s3"]}),
            serde_json::json!({"sid": "s3", "op": ["res_op", second, "drop"], "transfer": ["next", "s4"]}),
            serde_json::json!({"sid": "s4", "op": ["res_op", first, "drop"], "transfer": ["next", "s5"]}),
            serde_json::json!({"sid": "s5", "op": "return", "transfer": "return"}),
        ]
    };
    let program: concir::ast::Program = serde_json::from_value(serde_json::json!({
        "program": "mod_deadlock",
        "resources": [
            {"name": "m1", "kind": "sync", "type": "Mutex", "mode": "Sync"},
            {"name": "m2", "kind": "sync", "type": "Mutex", "mode": "Sync"}
        ],
        "protection": [],
        "functions": [
            {
                "name": "main", "kind": "normal", "module": "main",
                "body": [
                    {"sid": "s1", "op": ["spawn", "w1"], "transfer": ["next", "s2"]},
                    {"sid": "s2", "op": ["spawn", "w2"], "transfer": ["next", "s3"]},
                    {"sid": "s3", "op": ["join", "w1"], "transfer": ["next", "s4"]},
                    {"sid": "s4", "op": ["join", "w2"], "transfer": ["next", "s5"]},
                    {"sid": "s5", "op": "return", "transfer": "return"}
                ]
            },
            {"name": "w1", "kind": "closure", "module": "alpha", "body": lock_body(("m1", "m2"))},
            {"name": "w2", "kind": "closure", "module": "beta", "body": lock_body(("m2", "m1"))}
        ],
        "entry": "main"
    }))
    .expect("test CIR must parse");

    let result = verify_program(&program, &VerificationConfig::default());
    assert_eq!(result.status, VerificationStatus::VerifiedUnsafe);
    assert_eq!(result.bugs.len(), 1);

    let bug = &result.bugs[0];
    assert_eq!(bug.involved_modules, vec!["alpha", "beta", "main"]);
    // Every trace step carries the module of its source function.
    let trace_modules: Vec<&str> = bug
        .trace
        .iter()
        .filter_map(|s| s.module.as_deref())
        .collect();
    assert!(!trace_modules.is_empty());
    // The CIR slice entries are module-tagged too.
    assert!(bug.cir_slice.iter().all(|e| e.module.is_some()));
}
