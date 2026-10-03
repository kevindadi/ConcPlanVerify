//! The semaphore initial count comes from the construction that is used.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::{json, Value};

fn tmp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let base = std::env::temp_dir();
    for _ in 0..100 {
        let id = SEQ.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = base.join(format!("sem_init_{name}_{}_{}_{}", std::process::id(), id, nanos));
        match fs::create_dir(&dir) {
            Ok(()) => return dir,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => panic!("create {dir:?}: {err}"),
        }
    }
    panic!("could not create a unique temp directory")
}

fn check(src: &str, count: i64) -> Value {
    let dir = tmp("bind");
    let input = dir.join("input.rs");
    fs::write(&input, src).unwrap();
    let out = dir.join("out");
    let inst = Command::new(env!("CARGO_BIN_EXE_concir-instrument"))
        .args([input.to_str().unwrap(), "--out", out.to_str().unwrap(), "--wrappers"])
        .output()
        .unwrap();
    assert!(inst.status.success(), "{}", String::from_utf8_lossy(&inst.stderr));
    let cir = json!({
        "program": "sem",
        "version": "3.5.0",
        "entry": "main::main",
        "modules": [{
            "name": "main",
            "resources": [{"name": "g12", "kind": "sync", "type": "Semaphore", "mode": "Sync", "count": count}],
            "protection": [],
            "functions": [{"name": "main", "kind": "normal", "body": [{"sid": "s1", "kind": "return"}]}]
        }]
    });
    let cir_path = dir.join("cir.json");
    fs::write(&cir_path, serde_json::to_string(&cir).unwrap()).unwrap();
    let bind = Command::new(env!("CARGO_BIN_EXE_bind_check"))
        .args(["--resources", out.join("resources.json").to_str().unwrap(), "--cir", cir_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(bind.status.success(), "{}", String::from_utf8_lossy(&bind.stderr));
    serde_json::from_slice(&bind.stdout).unwrap()
}

fn attr<'a>(doc: &'a Value) -> &'a Value {
    doc["attributes"].as_array().unwrap().iter()
        .find(|row| row["resource_id"] == "main::g12")
        .unwrap_or_else(|| panic!("no g12 attribute: {doc}"))
}

const HEADER: &str = "use concir_sync::Semaphore;\n";

#[test]
fn used_constructor_wins_over_an_earlier_same_name() {
    let src = format!("{HEADER}fn main() {{ let g12 = Semaphore::new(0); let g12 = Semaphore::new(2); g12.release_count(1).unwrap(); }}\n");
    let doc = check(&src, 2);
    let row = attr(&doc);
    assert_eq!(row["status"], "match", "{doc}");
    assert_eq!(row["observed"], 2, "{doc}");
    let wrong = check(&src, 0);
    assert_eq!(attr(&wrong)["status"], "mismatch", "{wrong}");
    assert_eq!(attr(&wrong)["observed"], 2, "{wrong}");
}

#[test]
fn alias_does_not_keep_the_unused_constructor() {
    let src = format!(
        "{HEADER}fn main() {{ let spare = Semaphore::new(2); let g12 = Semaphore::new(0); let g12 = spare; g12.release_count(1).unwrap(); }}\n"
    );
    let doc = check(&src, 0);
    let row = attr(&doc);
    assert_eq!(row["status"], "unknown", "{doc}");
    assert_ne!(row["observed"], 0, "{doc}");
}

#[test]
fn closure_tuple_uses_the_cloned_construction() {
    let src = r#"
use concir_sync::Semaphore;
use std::sync::Arc;
fn w(g12: Arc<Semaphore>) { g12.release_count(1).unwrap(); }
fn main() {
    let g12 = Semaphore::new(9);
    let g12 = Semaphore::new(0);
    let mk = || (Arc::clone(&g12),);
    let (a,) = mk();
    w(a);
}
"#;
    let doc = check(src, 0);
    assert_eq!(attr(&doc)["status"], "match", "{doc}");
    assert_eq!(attr(&doc)["observed"], 0, "{doc}");
    let wrong = check(src, 9);
    assert_eq!(attr(&wrong)["status"], "mismatch", "{wrong}");
}

#[test]
fn unsupported_initial_is_unknown() {
    let src = format!(
        "{HEADER}fn main() {{ let g12 = if true {{ Semaphore::new(0) }} else {{ Semaphore::new(2) }}; g12.release_count(1).unwrap(); }}\n"
    );
    let doc = check(&src, 0);
    assert_eq!(attr(&doc)["status"], "unknown", "{doc}");
}

#[test]
fn ignored_work_value_does_not_invalidate_worker_semaphore() {
    let src = r#"
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;
fn w(g12: Arc<Semaphore>) {
    let mut work = 0;
    let permit = g12.acquire();
    work += 1;
    permit.release();
    let _ = work;
}
fn main() {
    let g12 = Semaphore::new(1);
    let s = Arc::clone(&g12);
    thread::spawn(move || w(s)).join().unwrap();
}
"#;
    let doc = check(src, 1);
    assert_eq!(attr(&doc)["status"], "match", "{doc}");
    let wrong = check(src, 2);
    assert_eq!(attr(&wrong)["status"], "mismatch", "{wrong}");
}

#[test]
fn method_clone_keeps_the_known_arc_constructor() {
    let src = r#"
use concir_sync::Semaphore;
fn w(s: &Semaphore) { let p = s.acquire(); p.release(); }
fn main() {
    let g12 = Semaphore::new(1);
    let a = g12.clone();
    std::thread::spawn(move || w(&a)).join().unwrap();
}
"#;
    assert_eq!(attr(&check(src, 1))["status"], "match");
    assert_eq!(attr(&check(src, 2))["status"], "mismatch");
}

#[test]
fn method_clone_cannot_recover_an_unknown_shadowed_receiver() {
    let src = r#"
use concir_sync::Semaphore;
fn unknown() -> std::sync::Arc<Semaphore> { Semaphore::new(2) }
fn main() {
    let g12 = Semaphore::new(1);
    let g12 = unknown();
    let a = g12.clone();
    let p = a.acquire();
    p.release();
}
"#;
    assert_eq!(attr(&check(src, 1))["status"], "unknown");
}
