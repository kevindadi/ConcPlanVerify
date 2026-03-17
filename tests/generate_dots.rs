//! Generate DOT visualizations for all CIR fixtures and examples.
//!
//! Run with: cargo test --test generate_dots -- --ignored
//!
//! Outputs to:
//!   dots/cir/   — CIR control-flow DOT files
//!   dots/cvn/   — CVN Petri-net DOT files (only for translatable inputs)

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use cir::ast::Program;

fn output_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("dots")
}

fn load_json(path: &Path) -> Program {
    let json = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn generate_for(json_path: &Path, stem: &str) {
    let program = load_json(json_path);
    let cir_dir = output_dir().join("cir");
    let cvn_dir = output_dir().join("cvn");
    fs::create_dir_all(&cir_dir).unwrap();
    fs::create_dir_all(&cvn_dir).unwrap();

    // CIR DOT
    let cir_dot = program.to_dot();
    let cir_path = cir_dir.join(format!("{stem}.dot"));
    fs::write(&cir_path, &cir_dot).unwrap();
    eprintln!("  wrote {}", cir_path.display());

    // CVN DOT (translate, skip on error)
    match cir2cvn::translate(&program) {
        Ok(net) => {
            let cvn_dot = cvn::export::to_dot(&net);
            let cvn_path = cvn_dir.join(format!("{stem}.dot"));
            fs::write(&cvn_path, &cvn_dot).unwrap();
            eprintln!("  wrote {}", cvn_path.display());
        }
        Err(errs) => {
            eprintln!(
                "  skip CVN for {stem}: {}",
                errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; ")
            );
        }
    }
}

#[test]
#[ignore]
fn generate_all_dots() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    eprintln!("\n=== Test fixtures ===");
    let fixtures_dir = root.join("tests/fixtures");
    if fixtures_dir.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(&fixtures_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let stem = path.file_stem().unwrap().to_str().unwrap();
            eprintln!("[fixture] {stem}");
            generate_for(&path, &format!("fixture_{stem}"));
        }
    }

    eprintln!("\n=== CIR examples ===");
    let examples_dir = root.join("cir/examples");
    if examples_dir.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(&examples_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let stem = path.file_stem().unwrap().to_str().unwrap();
            eprintln!("[example] {stem}");
            generate_for(&path, &format!("example_{stem}"));
        }
    }

    eprintln!("\nDone! DOT files written to {}", output_dir().display());
}
