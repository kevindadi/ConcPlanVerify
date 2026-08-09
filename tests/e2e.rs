//! End-to-end tests: ConcIR (buggy) → translate → explore → assert bug → fixed → verify

use std::path::Path;

use concir::ast::Program;
use cvn::analysis::{check_goals, explore, find_dead_transitions, AnalysisConfig};
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
            prompt.contains("## Current ConcIR"),
            "[{dir_name}] prompt missing ConcIR section"
        );
        assert!(
            prompt.contains("## Repair Strategy") || prompt.contains("Repair Strategy"),
            "[{dir_name}] prompt missing repair strategy"
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

/// Run a buggy fixture whose oracle is *goal unreachability* rather than a
/// concrete deadlock:
/// 1. the CVN search must find no deadlock (the bug is a partial deadlock /
///    behaviour regression, not a real deadlock), and
/// 2. every business goal declared in the ConcIR must be reported as unmet by
///    [`cvn::analysis::check_goals`].
fn run_goal_buggy_test(dir_name: &str) {
    let dir = e2e_dir().join(dir_name);
    let buggy = load_cir(&dir.join("buggy.json"));
    assert!(
        !buggy.goals.is_empty(),
        "[{dir_name}/buggy] fixture must declare at least one business goal"
    );

    let net = translate(&buggy);
    let config = AnalysisConfig::default();
    let result = explore(&net, &config).expect("state space exploration should succeed");

    assert!(
        result.deadlocks.is_empty(),
        "[{dir_name}/buggy] expected a *partial* deadlock (no CVN deadlock), but found {}",
        result.deadlocks.len()
    );

    let (specs, warnings) = cir2cvn::translate_goals(&buggy);
    assert!(
        warnings.is_empty(),
        "[{dir_name}/buggy] goal translation warnings: {warnings:?}"
    );
    let unmet = check_goals(&net, &specs, &config).expect("goal check should succeed");
    assert_eq!(
        unmet.len(),
        specs.len(),
        "[{dir_name}/buggy] expected all {} goals to be unreachable, but only {} were reported",
        specs.len(),
        unmet.len()
    );
}

/// Run a fixed fixture whose oracle is: no deadlock AND every declared
/// business goal is reachable.
fn run_goal_fixed_test(dir_name: &str) {
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
        "[{dir_name}/fixed] expected no deadlocks but found {}",
        result.deadlocks.len()
    );

    let (specs, warnings) = cir2cvn::translate_goals(&fixed);
    assert!(
        warnings.is_empty(),
        "[{dir_name}/fixed] goal translation warnings: {warnings:?}"
    );
    let unmet = check_goals(&net, &specs, &config).expect("goal check should succeed");
    assert!(
        unmet.is_empty(),
        "[{dir_name}/fixed] expected all goals reachable, but {} remained unmet: {:?}",
        unmet.len(),
        unmet.iter().map(|g| &g.goal.id).collect::<Vec<_>>()
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

/// Regression: the fixed fixture signals via `notify_all`, so the
/// wake-by-notify variant of the waiter's `wait` never fires. That is a
/// translation artifact, not a defect — the full pipeline must report
/// `verified_safe` instead of a DeadTransition false positive.
#[test]
fn e2e_signal_loss_fixed_full_pipeline_is_safe() {
    let fixed = load_cir(&e2e_dir().join("signal_loss/fixed.json"));
    let result = cir2cvn::verify_program(&fixed, &cir2cvn::VerificationConfig::default());
    assert_eq!(
        result.status,
        cir2cvn::VerificationStatus::VerifiedSafe,
        "bugs: {:?}",
        result.bugs.iter().map(|b| &b.summary).collect::<Vec<_>>()
    );
}

#[test]
fn e2e_signal_loss_buggy_repair_feedback_filters_deadlock_suffixes() {
    let program = load_cir(&e2e_dir().join("signal_loss/buggy.json"));
    let net = translate(&program);
    let result = explore(&net, &AnalysisConfig::default()).expect("state space exploration should succeed");

    let reports = cir2cvn::repair::analyze(&program, &net, &result);
    assert!(
        reports
            .iter()
            .any(|report| matches!(&report.kind, cir2cvn::repair::BugKind::SignalLoss { .. })),
        "repair feedback should retain the primary SignalLoss report: {:?}",
        reports.iter().map(|report| report.kind.name()).collect::<Vec<_>>()
    );
    assert!(
        reports.iter().all(|report| {
            !matches!(
                &report.kind,
                cir2cvn::repair::BugKind::DeadTransition { .. }
            )
        }),
        "signal-loss feedback should contain no independent DeadTransition targets: {:?}",
        reports.iter().map(|report| report.kind.name()).collect::<Vec<_>>()
    );
}

#[test]
fn e2e_signal_loss_fixed_full_pipeline_has_no_bugs() {
    let fixed = load_cir(&e2e_dir().join("signal_loss/fixed.json"));
    let result = cir2cvn::verify_program(&fixed, &cir2cvn::VerificationConfig::default());
    assert_eq!(
        result.status,
        cir2cvn::VerificationStatus::VerifiedSafe,
        "bugs: {:?}",
        result.bugs.iter().map(|b| &b.summary).collect::<Vec<_>>()
    );
    assert!(result.bugs.is_empty());
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

// ── Test 7: Partial Deadlock (goal-reachability oracle) ──

#[test]
fn e2e_partial_deadlock_buggy() {
    run_goal_buggy_test("partial_deadlock");
}

#[test]
fn e2e_partial_deadlock_fixed() {
    run_goal_fixed_test("partial_deadlock");
}

// ── Test 8: Dual Condvar (genuine deadlock) ──

#[test]
fn e2e_dual_condvar_buggy() {
    run_buggy_test("dual_condvar");
}

#[test]
fn e2e_dual_condvar_fixed() {
    run_fixed_test("dual_condvar");
}

#[test]
fn e2e_dual_condvar_buggy_repair_feedback_filters_deadlock_suffixes() {
    let program = load_cir(&e2e_dir().join("dual_condvar/buggy.json"));
    let net = translate(&program);
    let result = explore(&net, &AnalysisConfig::default()).expect("state space exploration should succeed");

    let raw_dead_transitions = find_dead_transitions(&net, &result);
    assert!(
        raw_dead_transitions.iter().any(|cx| matches!(
            &cx.kind,
            cvn::analysis::PropertyViolation::DeadTransition { .. }
        )),
        "raw CVN analysis should retain downstream dead-transition observations"
    );

    let reports = cir2cvn::repair::analyze(&program, &net, &result);
    assert!(
        reports
            .iter()
            .any(|report| matches!(&report.kind, cir2cvn::repair::BugKind::SignalLoss { .. })),
        "repair feedback should retain the primary SignalLoss report: {:?}",
        reports.iter().map(|report| report.kind.name()).collect::<Vec<_>>()
    );
    assert!(
        reports.iter().all(|report| {
            !matches!(
                &report.kind,
                cir2cvn::repair::BugKind::DeadTransition { .. }
            )
        }),
        "deadlock suffixes should not become independent repair targets: {:?}",
        reports.iter().map(|report| report.kind.name()).collect::<Vec<_>>()
    );
}

#[test]
fn e2e_dual_condvar_fixed_full_pipeline_is_safe() {
    let fixed = load_cir(&e2e_dir().join("dual_condvar/fixed.json"));
    let result = cir2cvn::verify_program(&fixed, &cir2cvn::VerificationConfig::default());
    assert_eq!(
        result.status,
        cir2cvn::VerificationStatus::VerifiedSafe,
        "bugs: {:?}",
        result.bugs.iter().map(|b| &b.summary).collect::<Vec<_>>()
    );
    assert!(result.bugs.is_empty());
}

// ── Test 8b: Condvar wait while holding another mutex ──

#[test]
fn e2e_condvar_lock_hold_buggy() {
    run_buggy_test("condvar_lock_hold");
}

#[test]
fn e2e_condvar_lock_hold_fixed() {
    run_fixed_test("condvar_lock_hold");
}

#[test]
fn e2e_condvar_lock_hold_fixed_full_pipeline_is_safe() {
    let fixed = load_cir(&e2e_dir().join("condvar_lock_hold/fixed.json"));
    let result = cir2cvn::verify_program(&fixed, &cir2cvn::VerificationConfig::default());
    assert_eq!(
        result.status,
        cir2cvn::VerificationStatus::VerifiedSafe,
        "bugs: {:?}",
        result.bugs.iter().map(|b| &b.summary).collect::<Vec<_>>()
    );
}

// ── Test 9: FnSummary propagation baseline (no bug, goals reachable) ──

#[test]
fn e2e_fn_summary_prop_no_bug() {
    run_goal_fixed_test("fn_summary_prop");
}

// ── Goals trust boundary: weak / dangling goal specs must be rejected ──

fn verify_benchmark(rel_path: &str) -> cir2cvn::VerificationResult {
    let program = load_cir(&Path::new(env!("CARGO_MANIFEST_DIR")).join(rel_path));
    cir2cvn::verify_program(&program, &cir2cvn::VerificationConfig::default())
}

/// A goal already satisfied by the initial state constrains nothing and must
/// be flagged instead of silently passing.
#[test]
fn goal_trivial_buggy_is_flagged_as_too_weak() {
    let result = verify_benchmark("benchmarks/cir/goal_trivial/buggy.json");
    assert_eq!(result.status, cir2cvn::VerificationStatus::GoalsUnmet);
    assert!(
        result.goal_warnings.iter().any(|w| w.contains("initial state")),
        "expected triviality warning, got: {:?}",
        result.goal_warnings
    );
}

#[test]
fn goal_trivial_fixed_is_safe() {
    let result = verify_benchmark("benchmarks/cir/goal_trivial/fixed.json");
    assert_eq!(result.status, cir2cvn::VerificationStatus::VerifiedSafe);
}

/// A goal referencing an unknown marking key or an undeclared variable must
/// produce warnings, not a vacuously passing (or vacuously failing) check.
#[test]
fn goal_bad_reference_buggy_is_flagged() {
    let result = verify_benchmark("benchmarks/cir/goal_bad_reference/buggy.json");
    assert_eq!(result.status, cir2cvn::VerificationStatus::GoalsUnmet);
    assert!(
        result
            .goal_warnings
            .iter()
            .any(|w| w.contains("marking key 'w1_done'")),
        "expected unknown-marking warning, got: {:?}",
        result.goal_warnings
    );
    assert!(
        result
            .goal_warnings
            .iter()
            .any(|w| w.contains("not a declared 'var' resource")),
        "expected unknown-variable warning, got: {:?}",
        result.goal_warnings
    );
}

#[test]
fn goal_bad_reference_fixed_is_safe() {
    let result = verify_benchmark("benchmarks/cir/goal_bad_reference/fixed.json");
    assert_eq!(result.status, cir2cvn::VerificationStatus::VerifiedSafe);
}

// ── Test 10: Dead Transition (behavioral unreachability) ──

/// Helper: a fixture is "dead-transition-free" iff
/// [`find_dead_transitions`] returns the empty set on its state graph.
fn run_dead_transition_fixed_test(dir_name: &str) {
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
        "[{dir_name}/fixed] expected no deadlocks but found {}",
        result.deadlocks.len()
    );

    let dead = find_dead_transitions(&net, &result);
    assert!(
        dead.is_empty(),
        "[{dir_name}/fixed] expected no dead transitions, but found {}: {:?}",
        dead.len(),
        dead.iter()
            .map(|cx| match &cx.kind {
                cvn::analysis::PropertyViolation::DeadTransition { transition_id, .. } =>
                    transition_id.0.clone(),
                _ => "<other>".into(),
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn e2e_dead_transition_buggy() {
    run_buggy_test("dead_transition");
}

#[test]
fn e2e_dead_transition_buggy_repair_feedback_retains_independent_report() {
    let program = load_cir(&e2e_dir().join("dead_transition/buggy.json"));
    let net = translate(&program);
    let result = explore(&net, &AnalysisConfig::default()).expect("state space exploration should succeed");

    let raw_dead_transitions = find_dead_transitions(&net, &result);
    assert!(
        !raw_dead_transitions.is_empty(),
        "raw CVN analysis should find the unreachable branch"
    );

    let reports = cir2cvn::repair::analyze(&program, &net, &result);
    assert!(
        reports.iter().any(|report| matches!(
            &report.kind,
            cir2cvn::repair::BugKind::DeadTransition { .. }
        )),
        "independent dead transition should remain repairable feedback: {:?}",
        reports.iter().map(|report| report.kind.name()).collect::<Vec<_>>()
    );
}

#[test]
fn e2e_dead_transition_fixed() {
    run_dead_transition_fixed_test("dead_transition");
}
