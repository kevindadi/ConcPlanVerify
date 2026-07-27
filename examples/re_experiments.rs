use std::fs;
use std::path::Path;
use std::time::Instant;

use cir::ast::{Op, Program};
use cir2cvn::{VerificationConfig, VerificationStatus, translate, verify_program};
use cvn::analysis::{AnalysisConfig, SearchStrategy, explore};
use serde::Serialize;
use serde_json::{Value, json};

const MAX_STATES: usize = 100_000;
const TIMING_RUNS: usize = 3;

#[derive(Serialize)]
struct GoalAblationRow {
    artifact: String,
    full_status: String,
    no_goal_status: String,
    states: usize,
    bugs: usize,
    unmet_goals: usize,
    accepted_without_goals: bool,
}

#[derive(Serialize)]
struct GoalAblationResults {
    configuration: Value,
    benchmark: Vec<GoalAblationRow>,
    controlled_mutation: Vec<GoalAblationRow>,
}

#[derive(Serialize)]
struct ScalingRow {
    family: String,
    size: usize,
    threads: usize,
    locks: usize,
    condvars: usize,
    statements: usize,
    places: usize,
    transitions: usize,
    states: Option<usize>,
    reached_cap: bool,
    median_search_ms: f64,
}

#[derive(Serialize)]
struct ScalingResults {
    configuration: Value,
    rows: Vec<ScalingRow>,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let experiment = args.next().unwrap_or_else(|| usage());
    let output = match (args.next().as_deref(), args.next()) {
        (None, None) => None,
        (Some("--output"), Some(path)) => Some(path),
        _ => usage(),
    };

    let rendered = match experiment.as_str() {
        "goal-ablation" => serde_json::to_string_pretty(&run_goal_ablation()).unwrap(),
        "scaling" => serde_json::to_string_pretty(&run_scaling()).unwrap(),
        _ => usage(),
    };

    if let Some(path) = output {
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, rendered).unwrap();
    } else {
        println!("{rendered}");
    }
}

fn usage() -> ! {
    eprintln!(
        "usage: cargo run --release --example rebuttal_experiments -- \
         <goal-ablation|scaling> [--output PATH]"
    );
    std::process::exit(2);
}

fn run_goal_ablation() -> GoalAblationResults {
    let artifacts = [
        (
            "P1 two-mutex deadlock",
            "tests/e2e/mutex_deadlock/buggy.json",
        ),
        (
            "P2 condition-variable signal loss",
            "tests/e2e/signal_loss/buggy.json",
        ),
        (
            "P3 channel plus mutex",
            "tests/e2e/channel_deadlock/buggy.json",
        ),
        (
            "P4 three-lock cycle",
            "tests/e2e/three_way_deadlock/buggy.json",
        ),
        (
            "P5 partial deadlock",
            "tests/e2e/partial_deadlock/buggy.json",
        ),
        (
            "P6 dual condition variable",
            "tests/e2e/dual_condvar/buggy.json",
        ),
        (
            "P7 semaphore baseline",
            "tests/e2e/semaphore_throttle/buggy.json",
        ),
        ("P8 CAS baseline", "tests/e2e/cas_race/buggy.json"),
        (
            "P9 function-summary baseline",
            "tests/e2e/fn_summary_prop/fixed.json",
        ),
    ];

    let benchmark = artifacts
        .iter()
        .map(|(name, path)| {
            let program = read_program(path);
            compare_goal_configs(name, &program)
        })
        .collect();

    let mut original = read_program("tests/e2e/fn_summary_prop/fixed.json");
    original.goals.push(cir::ast::BusinessGoal {
        id: "g_result_written".into(),
        desc: Some("The producer writes the required result".into()),
        marking: Default::default(),
        variables: [("result".into(), json!(1))].into_iter().collect(),
    });
    let mut behavior_dropped = original.clone();
    let producer = behavior_dropped
        .functions
        .iter_mut()
        .find(|function| function.name == "producer")
        .unwrap();
    let write = producer
        .body
        .iter_mut()
        .find(|statement| statement.sid == "s3")
        .unwrap();
    write.op = Op::Nop;

    let controlled_mutation = vec![
        compare_goal_configs("goal-bearing reference", &original),
        compare_goal_configs(
            "behavior-dropping mutant (producer.s3 write -> nop)",
            &behavior_dropped,
        ),
    ];

    GoalAblationResults {
        configuration: json!({
            "max_states": MAX_STATES,
            "dead_transition_analysis": true,
            "full": {"check_goals": true},
            "ablation": {"check_goals": false},
            "note": "All other verifier settings are identical."
        }),
        benchmark,
        controlled_mutation,
    }
}

fn compare_goal_configs(name: &str, program: &Program) -> GoalAblationRow {
    let full = verify_program(program, &VerificationConfig::default());
    let no_goals = verify_program(
        program,
        &VerificationConfig {
            check_goals: false,
            ..VerificationConfig::default()
        },
    );
    GoalAblationRow {
        artifact: name.into(),
        full_status: status_name(full.status).into(),
        no_goal_status: status_name(no_goals.status).into(),
        states: full.state_count,
        bugs: full.bugs.len(),
        unmet_goals: full.unmet_goals.len(),
        accepted_without_goals: no_goals.status == VerificationStatus::VerifiedSafe,
    }
}

fn status_name(status: VerificationStatus) -> &'static str {
    match status {
        VerificationStatus::InvalidModel => "invalid_model",
        VerificationStatus::TranslationFailed => "translation_failed",
        VerificationStatus::AnalysisIncomplete => "analysis_incomplete",
        VerificationStatus::VerifiedSafe => "verified_safe",
        VerificationStatus::VerifiedUnsafe => "verified_unsafe",
        VerificationStatus::GoalsUnmet => "goals_unmet",
    }
}

fn run_scaling() -> ScalingResults {
    let mut rows = Vec::new();

    for size in 2..=12 {
        let row = measure_scaling("lock ring", size, ring_lock_program(size), size, 0);
        let reached_cap = row.reached_cap;
        rows.push(row);
        if reached_cap {
            break;
        }
    }

    for size in 1..=6 {
        let row = measure_scaling(
            "independent condvar handshakes",
            size,
            condvar_program(size),
            size,
            size,
        );
        let reached_cap = row.reached_cap;
        rows.push(row);
        if reached_cap {
            break;
        }
    }

    ScalingResults {
        configuration: json!({
            "build": "cargo --release",
            "strategy": "breadth-first search",
            "max_states": MAX_STATES,
            "timing_runs": TIMING_RUNS,
            "reported_time": "median state-space search time; translation excluded",
            "hardware": "Apple M4 Pro, 24 GB"
        }),
        rows,
    }
}

fn measure_scaling(
    family: &str,
    size: usize,
    program: Program,
    locks: usize,
    condvars: usize,
) -> ScalingRow {
    let validation = cir::validate::validate(&program);
    assert!(
        validation.valid,
        "generated {family} size {size} is invalid: {validation:?}"
    );
    let net = translate(&program).unwrap();
    let config = AnalysisConfig {
        strategy: SearchStrategy::Bfs,
        max_states: MAX_STATES,
    };
    let mut timings = Vec::with_capacity(TIMING_RUNS);
    let mut states = None;
    let mut reached_cap = false;

    for _ in 0..TIMING_RUNS {
        let started = Instant::now();
        match explore(&net, &config) {
            Ok(result) => states = Some(result.state_count),
            Err(error) if error.to_string().contains("state space explosion") => {
                reached_cap = true;
            }
            Err(error) => panic!("analysis failed for {family} size {size}: {error}"),
        }
        timings.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    timings.sort_by(f64::total_cmp);

    ScalingRow {
        family: family.into(),
        size,
        threads: if family == "lock ring" {
            size
        } else {
            size * 2
        },
        locks,
        condvars,
        statements: program
            .functions
            .iter()
            .map(|function| function.body.len())
            .sum(),
        places: net.place_count(),
        transitions: net.transition_count(),
        states,
        reached_cap,
        median_search_ms: timings[TIMING_RUNS / 2],
    }
}

fn ring_lock_program(size: usize) -> Program {
    let resources: Vec<Value> = (0..size)
        .map(|index| {
            json!({
                "name": format!("m{index}"),
                "kind": "sync",
                "type": "Mutex",
                "mode": "Sync"
            })
        })
        .collect();
    let mut main_body = Vec::new();
    for index in 0..size {
        main_body.push(statement(
            index + 1,
            json!(["spawn", format!("w{index}")]),
            index + 2,
        ));
    }
    for index in 0..size {
        main_body.push(statement(
            size + index + 1,
            json!(["join", format!("w{index}")]),
            size + index + 2,
        ));
    }
    main_body.push(return_statement(size * 2 + 1));

    let mut functions = vec![json!({"name": "main", "kind": "normal", "body": main_body})];
    for index in 0..size {
        let next = (index + 1) % size;
        functions.push(json!({
            "name": format!("w{index}"),
            "kind": "closure",
            "body": [
                statement(1, json!(["res_op", format!("m{index}"), "lock"]), 2),
                statement(2, json!(["res_op", format!("m{next}"), "lock"]), 3),
                statement(3, json!(["res_op", format!("m{next}"), "drop"]), 4),
                statement(4, json!(["res_op", format!("m{index}"), "drop"]), 5),
                return_statement(5)
            ]
        }));
    }
    parse_program(json!({
        "program": format!("ring_lock_{size}"),
        "resources": resources,
        "protection": [],
        "functions": functions,
        "fn_summaries": [],
        "entry": "main",
        "goals": []
    }))
}

fn condvar_program(size: usize) -> Program {
    let mut resources = Vec::new();
    for index in 0..size {
        resources.push(json!({
            "name": format!("m{index}"),
            "kind": "sync",
            "type": "Mutex",
            "mode": "Sync"
        }));
        resources.push(json!({
            "name": format!("cv{index}"),
            "kind": "sync",
            "type": "Condvar",
            "mode": "Sync"
        }));
        resources.push(json!({
            "name": format!("ready{index}"),
            "kind": "var",
            "type": "Var",
            "base": "Bool",
            "init": false
        }));
    }
    let protection: Vec<Value> = (0..size)
        .map(|index| json!({"var": format!("ready{index}"), "lock": format!("m{index}")}))
        .collect();

    let mut main_body = Vec::new();
    for index in 0..size {
        main_body.push(statement(
            index * 2 + 1,
            json!(["spawn", format!("waiter{index}")]),
            index * 2 + 2,
        ));
        main_body.push(statement(
            index * 2 + 2,
            json!(["spawn", format!("notifier{index}")]),
            index * 2 + 3,
        ));
    }
    for index in 0..size {
        main_body.push(statement(
            size * 2 + index * 2 + 1,
            json!(["join", format!("waiter{index}")]),
            size * 2 + index * 2 + 2,
        ));
        main_body.push(statement(
            size * 2 + index * 2 + 2,
            json!(["join", format!("notifier{index}")]),
            size * 2 + index * 2 + 3,
        ));
    }
    main_body.push(return_statement(size * 4 + 1));

    let mut functions = vec![json!({"name": "main", "kind": "normal", "body": main_body})];
    for index in 0..size {
        functions.push(json!({
            "name": format!("waiter{index}"),
            "kind": "closure",
            "body": [
                statement(1, json!(["res_op", format!("m{index}"), "lock"]), 2),
                branch_statement(
                    2,
                    json!(["res_op", format!("ready{index}"), "read"]),
                    format!("ready{index} == true"),
                    4,
                    3,
                ),
                statement(
                    3,
                    json!(["res_op", format!("cv{index}"), "wait", format!("m{index}")]),
                    2,
                ),
                statement(4, json!(["res_op", format!("m{index}"), "drop"]), 5),
                return_statement(5)
            ]
        }));
        functions.push(json!({
            "name": format!("notifier{index}"),
            "kind": "closure",
            "body": [
                statement(1, json!(["res_op", format!("m{index}"), "lock"]), 2),
                statement(2, json!(["res_op", format!("ready{index}"), "write", "true"]), 3),
                statement(3, json!(["res_op", format!("cv{index}"), "notify_all"]), 4),
                statement(4, json!(["res_op", format!("m{index}"), "drop"]), 5),
                return_statement(5)
            ]
        }));
    }
    parse_program(json!({
        "program": format!("condvar_handshakes_{size}"),
        "resources": resources,
        "protection": protection,
        "functions": functions,
        "fn_summaries": [],
        "entry": "main",
        "goals": []
    }))
}

fn statement(sid: usize, op: Value, next_sid: usize) -> Value {
    json!({
        "sid": format!("s{sid}"),
        "op": op,
        "transfer": ["next", format!("s{next_sid}")]
    })
}

fn branch_statement(
    sid: usize,
    op: Value,
    condition: String,
    true_sid: usize,
    false_sid: usize,
) -> Value {
    json!({
        "sid": format!("s{sid}"),
        "op": op,
        "transfer": [
            "branch",
            condition,
            format!("s{true_sid}"),
            format!("s{false_sid}")
        ]
    })
}

fn return_statement(sid: usize) -> Value {
    json!({"sid": format!("s{sid}"), "op": "return", "transfer": "return"})
}

fn parse_program(value: Value) -> Program {
    serde_json::from_value(value).unwrap()
}

fn read_program(path: &str) -> Program {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}
