//! Actual runtime controls: returning, detached, and panicking workers.
use concir::instrument::wrap;
use std::fs;
use std::process::Command;
use serde_json::Value;

fn run(source: &str) -> (String, Vec<Value>) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let id = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("completion_{}_{}_{}", std::process::id(), id, nanos));
    fs::create_dir(&dir).unwrap();
    let wrapped = wrap(source).unwrap();
    fs::write(dir.join("main.rs"), wrapped.annotated).unwrap();
    fs::write(dir.join("cir_trace.rs"), wrapped.runtime).unwrap();
    // The generated runtime installs the real semaphore recorders, even when
    // this particular source uses no semaphore. Link the same workspace crate
    // as the Python candidate evaluator instead of stripping runtime code.
    let sync_crate = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().join("ConcPlanVerify/runtime/concir_sync");
    let manifest = format!(r#"[package]
name = "completion_probe"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "probe"
path = "main.rs"
[dependencies]
concir_sync = {{ path = "{}" }}
"#, sync_crate.display());
    fs::write(dir.join("Cargo.toml"), manifest).unwrap();
    let binary = dir.join("target/debug/probe");
    let built = Command::new("cargo").args(["build", "--offline", "--quiet", "--manifest-path"])
        .arg(dir.join("Cargo.toml")).output().unwrap();
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let trace = dir.join("trace.jsonl");
    let output = Command::new(binary).env("CIR_TRACE_OUT", &trace).output().unwrap();
    assert!(output.status.success());
    let events = fs::read_to_string(trace).unwrap().lines()
        .map(|line| serde_json::from_str(line).unwrap()).collect();
    let stdout = String::from_utf8(output.stdout).unwrap();
    fs::remove_dir_all(dir).unwrap();
    (stdout, events)
}

#[test]
fn normal_return_records_completion_and_preserves_output() {
    let (stdout, events) = run("fn worker() -> i32 { 7 } fn main() { let h = std::thread::spawn(worker); println!(\"{}\", h.join().unwrap()); }");
    assert_eq!(stdout.trim(), "7");
    let completes: Vec<_> = events.iter().filter(|e| e["op"] == "complete").collect();
    assert_eq!(completes.len(), 1);
    let spawned = events.iter().find(|e| e["op"] == "spawn").unwrap();
    assert_eq!(spawned["r"], completes[0]["r"]);
}

#[test]
fn detached_blocked_worker_has_no_completion() {
    let (_, events) = run("use std::sync::{Arc, Barrier}; fn worker(b: Arc<Barrier>) { b.wait(); } fn main() { let b = Arc::new(Barrier::new(2)); let _h = std::thread::spawn(move || worker(b)); }");
    assert!(events.iter().any(|e| e["op"] == "spawn"));
    assert!(!events.iter().any(|e| e["op"] == "complete"));
}

#[test]
fn caught_worker_panic_has_no_normal_completion() {
    let (_, events) = run("fn worker() { panic!(\"control\"); } fn main() { let h = std::thread::spawn(worker); assert!(h.join().is_err()); }");
    assert!(events.iter().any(|e| e["op"] == "spawn"));
    assert!(!events.iter().any(|e| e["op"] == "complete"));
}
