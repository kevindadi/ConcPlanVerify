//! `cir2cvn` CLI driver used by the experiment runner.
//!
//! Three subcommands are exposed, all reading CIR JSON either from a file
//! path or from stdin (`-`):
//!
//! * `--validate`  → run CIR static checks (58 rules). Exit 0 if valid.
//! * `--analyze`   → run the complete verification pipeline and print its
//!                   structured JSON result.
//! * `--goals`     → compatibility alias for the same complete verification
//!                   pipeline, including goal reachability.

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use cir::ast::Program;
use serde_json::json;

fn main() {
    process::exit(run());
}

fn run() -> i32 {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        return emit_error(
            "usage_error",
            "usage: cir2cvn (--validate|--analyze|--goals) <file.json | ->",
            2,
        );
    }

    let mode = &args[1];
    let input = &args[2];
    let source = match read_input(input) {
        Ok(s) => s,
        Err(e) => {
            return emit_error(
                "input_error",
                format!("error reading '{input}': {e}"),
                2,
            );
        }
    };

    match mode.as_str() {
        "--validate" => cmd_validate(&source),
        "--analyze" | "--goals" => cmd_verify(&source),
        _ => emit_error(
            "usage_error",
            "usage: cir2cvn (--validate|--analyze|--goals) <file.json | ->",
            2,
        ),
    }
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

fn parse_program(source: &str) -> Result<Program, String> {
    match serde_json::from_str(source) {
        Ok(p) => Ok(p),
        Err(e) => Err(format!("JSON parse error: {e}")),
    }
}

// ── --validate ──────────────────────────────────────────────────────────

fn cmd_validate(source: &str) -> i32 {
    let program: Program = match parse_program(source) {
        Ok(program) => program,
        Err(error) => {
            let payload = json!({
                "status": "invalid_json",
                "valid": false,
                "diagnostics": [{
                    "code": "E000",
                    "severity": "error",
                    "message": error,
                }]
            });
            println!("{}", serde_json::to_string(&payload).expect("JSON serialization"));
            return 2;
        }
    };
    let report = cir::validate::validate(&program);
    let status = if report.valid { "valid" } else { "invalid_model" };
    let payload = json!({
        "status": status,
        "valid": report.valid,
        "diagnostics": report.diagnostics,
    });
    println!("{}", serde_json::to_string(&payload).expect("JSON serialization"));
    if report.valid { 0 } else { 1 }
}

// ── --analyze / --goals ─────────────────────────────────────────────────

fn cmd_verify(source: &str) -> i32 {
    let program: Program = match parse_program(source) {
        Ok(program) => program,
        Err(error) => {
            let payload = json!({
                "status": "invalid_json",
                "error": error,
            });
            println!("{}", serde_json::to_string(&payload).expect("JSON serialization"));
            return 2;
        }
    };

    let result = cir2cvn::verify_program(&program, &cir2cvn::VerificationConfig::default());
    println!("{}", serde_json::to_string(&result).unwrap());

    match result.status {
        cir2cvn::VerificationStatus::VerifiedSafe => 0,
        cir2cvn::VerificationStatus::InvalidModel
        | cir2cvn::VerificationStatus::TranslationFailed
        | cir2cvn::VerificationStatus::AnalysisIncomplete
        | cir2cvn::VerificationStatus::VerifiedUnsafe
        | cir2cvn::VerificationStatus::GoalsUnmet => 1,
    }
}

fn emit_error(status: &str, message: impl Into<String>, exit_code: i32) -> i32 {
    let payload = json!({
        "status": status,
        "error": message.into(),
    });
    println!("{}", serde_json::to_string(&payload).expect("JSON serialization"));
    exit_code
}
