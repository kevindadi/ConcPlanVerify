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
        "--analyze" | "--goals" => cmd_verify(&source),
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

// ── --analyze / --goals ─────────────────────────────────────────────────

fn cmd_verify(source: &str) {
    let program: Program = parse_program(source);

    let result = cir2cvn::verify_program(
        &program,
        &cir2cvn::VerificationConfig::default(),
    );
    println!("{}", serde_json::to_string(&result).unwrap());

    let exit_code = match result.status {
        cir2cvn::VerificationStatus::VerifiedSafe => 0,
        cir2cvn::VerificationStatus::InvalidModel
        | cir2cvn::VerificationStatus::TranslationFailed
        | cir2cvn::VerificationStatus::AnalysisIncomplete
        | cir2cvn::VerificationStatus::VerifiedUnsafe
        | cir2cvn::VerificationStatus::GoalsUnmet => 1,
    };

    if exit_code != 0 {
        process::exit(exit_code);
    }
}
