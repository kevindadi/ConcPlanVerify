//! `cir2cvn` CLI driver used by the experiment runner.
//!
//! Three subcommands are exposed, all reading CIR JSON either from a file
//! path or from stdin (`-`):
//!
//! * `--validate`  → run CIR static checks (58 rules). Exit 0 if valid.
//! * `--analyze`   → translate + CVN state-space search + deadlock
//!                   classification; prints a JSON summary on stdout.
//! * `--goals`     → translate goals + run goal-reachability check;
//!                   prints unmet-goal summary on stdout.

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;
use std::time::Instant;

use cir::ast::Program;
use cvn::analysis::{AnalysisConfig, PropertyViolation, check_goals, explore};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        usage_and_exit();
    }

    let mode = &args[1];
    let input = &args[2];
    let source = match read_input(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading '{input}': {e}");
            process::exit(2);
        }
    };

    match mode.as_str() {
        "--validate" => cmd_validate(&source),
        "--analyze" => cmd_analyze(&source),
        "--goals" => cmd_goals(&source),
        _ => usage_and_exit(),
    }
}

fn usage_and_exit() -> ! {
    eprintln!("usage: cir2cvn (--validate|--analyze|--goals) <file.json | ->");
    process::exit(2);
}

fn read_input(arg: &str) -> io::Result<String> {
    if arg == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        fs::read_to_string(arg)
    }
}

fn parse_program(source: &str) -> Program {
    match serde_json::from_str(source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("JSON parse error: {e}");
            process::exit(2);
        }
    }
}

// ── --validate ──────────────────────────────────────────────────────────

fn cmd_validate(source: &str) {
    let program: Program = parse_program(source);
    let report = cir::validate::validate(&program);
    let json = serde_json::to_string(&report).unwrap();
    println!("{json}");
    if !report.valid {
        process::exit(1);
    }
}

// ── --analyze ───────────────────────────────────────────────────────────

fn cmd_analyze(source: &str) {
    let program: Program = parse_program(source);

    let net = match cir2cvn::translate(&program) {
        Ok(n) => n,
        Err(errs) => {
            let msgs: Vec<String> = errs.iter().map(|e| e.to_string()).collect();
            let out = serde_json::json!({
                "error": format!("translation failed: {}", msgs.join("; "))
            });
            println!("{}", out);
            process::exit(1);
        }
    };

    let config = AnalysisConfig::default();
    let t0 = Instant::now();
    let result = match explore(&net, &config) {
        Ok(r) => r,
        Err(e) => {
            let out = serde_json::json!({
                "error": format!("state space exploration failed: {e}"),
                "places": net.places().count(),
                "transitions": net.transitions().count(),
            });
            println!("{}", out);
            process::exit(1);
        }
    };
    let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let reports = cir2cvn::repair::analyze(&program, &net, &result);

    let bugs: Vec<serde_json::Value> = reports
        .iter()
        .map(|r| {
            serde_json::json!({
                "kind": r.kind.name(),
                "involved_resources": r.involved_resources,
                "involved_functions": r.involved_functions,
            })
        })
        .collect();

    let bug_reports: Vec<serde_json::Value> = reports
        .iter()
        .map(|r| {
            serde_json::json!({
                "kind": r.kind.name(),
                "text": cir2cvn::repair::render::render_text(r),
            })
        })
        .collect();

    let deadlock_count = result
        .deadlocks
        .iter()
        .filter(|cx| matches!(cx.kind, PropertyViolation::Deadlock))
        .count();
    let dead_transition_count = cvn::analysis::find_dead_transitions(&net, &result).len();

    let out = serde_json::json!({
        "places": net.places().count(),
        "transitions": net.transitions().count(),
        "states": result.state_count,
        "analysis_time_ms": elapsed_ms,
        "deadlock_count": deadlock_count,
        "dead_transition_count": dead_transition_count,
        "bugs": bugs,
        "bug_reports": bug_reports,
    });
    println!("{}", out);
}

// ── --goals ─────────────────────────────────────────────────────────────

fn cmd_goals(source: &str) {
    let program: Program = parse_program(source);

    let net = match cir2cvn::translate(&program) {
        Ok(n) => n,
        Err(errs) => {
            let msgs: Vec<String> = errs.iter().map(|e| e.to_string()).collect();
            let out = serde_json::json!({
                "error": format!("translation failed: {}", msgs.join("; "))
            });
            println!("{}", out);
            process::exit(1);
        }
    };

    let (specs, warnings) = cir2cvn::translate_goals(&program);
    if specs.is_empty() {
        let out = serde_json::json!({
            "goals_total": 0,
            "goals_met": 0,
            "goals_unmet": 0,
            "unmet": [],
            "warnings": warnings,
        });
        println!("{}", out);
        return;
    }

    let config = AnalysisConfig::default();
    let unmet = match check_goals(&net, &specs, &config) {
        Ok(u) => u,
        Err(e) => {
            let out = serde_json::json!({
                "error": format!("goal reachability check failed: {e}"),
            });
            println!("{}", out);
            process::exit(1);
        }
    };

    let unmet_json: Vec<serde_json::Value> = unmet
        .iter()
        .map(|u| {
            serde_json::json!({
                "id": u.goal.id,
                "desc": u.goal.desc,
            })
        })
        .collect();

    let total = specs.len();
    let met = total - unmet.len();
    let out = serde_json::json!({
        "goals_total": total,
        "goals_met": met,
        "goals_unmet": unmet.len(),
        "unmet": unmet_json,
        "warnings": warnings,
    });
    println!("{}", out);
}
