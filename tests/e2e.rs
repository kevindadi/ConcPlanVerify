//! End-to-end tests: CIR (buggy) → translate → explore → assert bug → fixed → verify

use std::path::Path;

use cir::ast::Program;
use cvn::analysis::{explore, AnalysisConfig};
use cvn::net::CvnNet;

use serde::Deserialize;

#[derive(Deserialize)]
struct ExpectedBug {
    kind: String,
    involved_resources: Vec<String>,
    involved_functions: Vec<String>,
}

fn e2e_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/e2e")
}

fn load_cir(path: &Path) -> Program {
    let json = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

fn translate(program: &Program) -> CvnNet {
    cir2cvn::translate(program).unwrap_or_else(|errs| {
        let msgs: Vec<String> = errs.iter().map(|e| e.to_string()).collect();
        panic!("translation failed: {}", msgs.join("; "));
    })
}

fn run_buggy_test(dir_name: &str) {
    let dir = e2e_dir().join(dir_name);
    let buggy_path = dir.join("buggy.json");
    let expected_path = dir.join("expected_bug.json");

    let buggy = load_cir(&buggy_path);
    let net = translate(&buggy);
    let config = AnalysisConfig::default();
    let result = explore(&net, &config).expect("state space exploration should succeed");

    if expected_path.exists() {
        let expected: ExpectedBug = serde_json::from_str(
            &std::fs::read_to_string(&expected_path).unwrap(),
        )
        .unwrap();

        let reports = cir2cvn::repair::analyze(&buggy, &net, &result);
        assert!(
            !reports.is_empty(),
            "[{dir_name}] expected bug of kind '{}' but no bugs detected",
            expected.kind
        );

        let report = &reports[0];
        assert_eq!(
            report.kind.name(),
            expected.kind,
            "[{dir_name}] bug kind mismatch"
        );

        for res in &expected.involved_resources {
            assert!(
                report.involved_resources.contains(res),
                "[{dir_name}] expected involved resource '{res}' not found in {:?}",
                report.involved_resources
            );
        }

        for func in &expected.involved_functions {
            assert!(
                report.involved_functions.contains(func),
                "[{dir_name}] expected involved function '{func}' not found in {:?}",
                report.involved_functions
            );
        }

        let text = cir2cvn::repair::render::render_text(report);
        assert!(!text.is_empty(), "[{dir_name}] rendered text is empty");
        assert!(
            text.contains("BUG:"),
            "[{dir_name}] rendered text missing BUG: header"
        );
        assert!(
            text.contains("SUGGESTION:"),
            "[{dir_name}] rendered text missing SUGGESTION:"
        );

        let cir_json = serde_json::to_string_pretty(&buggy).unwrap();
        let prompt = cir2cvn::repair::render::render_repair_prompt(report, &cir_json);
        assert!(
            prompt.contains("## 原始 CIR"),
            "[{dir_name}] prompt missing CIR section"
        );
        assert!(
            prompt.contains("## 修复指导"),
            "[{dir_name}] prompt missing repair guidance"
        );
    } else {
        assert!(
            result.deadlocks.is_empty(),
            "[{dir_name}] expected no bugs but found {} deadlocks",
            result.deadlocks.len()
        );
    }
}

fn run_fixed_test(dir_name: &str) {
    let dir = e2e_dir().join(dir_name);
    let fixed_path = dir.join("fixed.json");

    if !fixed_path.exists() {
        return;
    }

    let fixed = load_cir(&fixed_path);
    let net = translate(&fixed);
    let config = AnalysisConfig::default();
    let result = explore(&net, &config).expect("state space exploration should succeed");

    assert!(
        result.deadlocks.is_empty(),
        "[{dir_name}/fixed] expected no deadlocks but found {}. First deadlock trace len: {}",
        result.deadlocks.len(),
        result.deadlocks.first().map(|d| d.trace.len()).unwrap_or(0)
    );
}

// ── Test 1: Mutex Deadlock ──

#[test]
fn e2e_mutex_deadlock_buggy() {
    run_buggy_test("mutex_deadlock");
}

#[test]
fn e2e_mutex_deadlock_fixed() {
    run_fixed_test("mutex_deadlock");
}

// ── Test 2: Signal Loss ──

#[test]
fn e2e_signal_loss_buggy() {
    run_buggy_test("signal_loss");
}

#[test]
fn e2e_signal_loss_fixed() {
    run_fixed_test("signal_loss");
}

// ── Test 3: Channel + Mutex Deadlock ──

#[test]
fn e2e_channel_deadlock_buggy() {
    run_buggy_test("channel_deadlock");
}

#[test]
fn e2e_channel_deadlock_fixed() {
    run_fixed_test("channel_deadlock");
}

// ── Test 4: Three-way Deadlock ──

#[test]
fn e2e_three_way_deadlock_buggy() {
    run_buggy_test("three_way_deadlock");
}

#[test]
fn e2e_three_way_deadlock_fixed() {
    run_fixed_test("three_way_deadlock");
}

// ── Test 5: Semaphore Throttle (no bug) ──

#[test]
fn e2e_semaphore_throttle_no_bug() {
    run_buggy_test("semaphore_throttle");
}

// ── Test 6: CAS Race (no bug) ──

#[test]
fn e2e_cas_race_no_bug() {
    run_buggy_test("cas_race");
}
