//! Channel capacity is separate from resource-name identity.

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
        let dir = base.join(format!("chan_sem_{name}_{}_{}_{}", std::process::id(), id, nanos));
        match fs::create_dir(&dir) {
            Ok(()) => return dir,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => panic!("create {dir:?}: {err}"),
        }
    }
    panic!("could not create a unique temp directory")
}

fn write_cir(dir: &PathBuf, channels: &[(&str, i64)]) -> PathBuf {
    let resources: Vec<Value> = channels
        .iter()
        .map(|(name, cap)| {
            json!({"name": name, "kind": "sync", "type": "Channel", "mode": "Sync", "base": "Int", "capacity": cap})
        })
        .collect();
    let names: Vec<&str> = channels.iter().map(|(n, _)| *n).collect();
    let cir = json!({
        "program": "c", "version": "3.5.0", "entry": "main::main",
        "modules": [{
            "name": "main",
            "provides": {"resources": names, "functions": ["main", "s", "r"]},
            "requires": {"resources": [], "functions": []},
            "resources": resources,
            "protection": [],
            "functions": [
                {"name": "s", "kind": "normal", "body": [{"sid": "s1", "kind": "return"}]},
                {"name": "r", "kind": "normal", "body": [{"sid": "r1", "kind": "return"}]},
                {"name": "main", "kind": "normal", "body": [{"sid": "m1", "kind": "return"}]}
            ]
        }]
    });
    let path = dir.join("cir.json");
    fs::write(&path, serde_json::to_string(&cir).unwrap()).unwrap();
    path
}

fn check(src: &str, channels: &[(&str, i64)]) -> Value {
    let dir = tmp(&format!("c{}_{}", src.len(), channels.len()));
    let input = dir.join("input.rs");
    fs::write(&input, src).unwrap();
    let out = dir.join("out");
    let inst = Command::new(env!("CARGO_BIN_EXE_concir-instrument"))
        .args([input.to_str().unwrap(), "--out", out.to_str().unwrap(), "--wrappers"])
        .output()
        .unwrap();
    assert!(inst.status.success(), "instrument: {}", String::from_utf8_lossy(&inst.stderr));
    let cir = write_cir(&dir, channels);
    let resources = out.join("resources.json");
    let bind = Command::new(env!("CARGO_BIN_EXE_bind_check"))
        .args(["--resources", resources.to_str().unwrap(), "--cir", cir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(bind.status.success(), "bind: {}", String::from_utf8_lossy(&bind.stderr));
    serde_json::from_slice(&bind.stdout).unwrap()
}

fn attr<'a>(doc: &'a Value, resource: &str) -> &'a Value {
    doc["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["resource_id"] == resource)
        .unwrap_or_else(|| panic!("no attribute for {resource}: {doc}"))
}

const HEADER: &str = r#"
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
"#;

#[test]
fn unbounded_conflicts_with_capacity_zero() {
    let src = format!(
        "{HEADER}
        fn s(ch1: Sender<i32>, ch2: Receiver<i32>, m: Arc<Mutex<()>>) {{
            let _g = m.lock().unwrap();
            ch1.send(1).unwrap();
            let _ = ch2.recv().unwrap();
        }}
        fn r(ch1: Receiver<i32>, ch2: Sender<i32>, m: Arc<Mutex<()>>) {{
            let _g = m.lock().unwrap();
            let _ = ch1.recv().unwrap();
            ch2.send(1).unwrap();
        }}
        fn main() {{
            let m = Arc::new(Mutex::new(()));
            let (ch1_tx, ch1_rx) = channel::<i32>();
            let (ch2_tx, ch2_rx) = channel::<i32>();
            let m_s = Arc::clone(&m);
            let m_r = Arc::clone(&m);
            thread::spawn(move || s(ch1_tx, ch2_rx, m_s));
            thread::spawn(move || r(ch1_rx, ch2_tx, m_r));
        }}
        "
    );
    let doc = check(&src, &[("ch1", 0), ("ch2", 0)]);
    assert_eq!(attr(&doc, "main::ch1")["status"], "mismatch");
    assert_eq!(attr(&doc, "main::ch1")["observed"], "unbounded");
    assert_eq!(attr(&doc, "main::ch1")["expected"], "rendezvous");
    assert_eq!(attr(&doc, "main::ch2")["status"], "mismatch");
    assert!(attr(&doc, "main::ch1")["construction_site"].is_string());
    assert_eq!(attr(&doc, "main::ch1")["evidence"], "param_from_call");
}

#[test]
fn rendezvous_matches_capacity_zero() {
    let src = format!(
        "{HEADER}
        fn s(ch1: SyncSender<i32>, ch2: Receiver<i32>, m: Arc<Mutex<()>>) {{
            ch1.send(1).unwrap();
            let _ = ch2.recv().unwrap();
        }}
        fn r(ch1: Receiver<i32>, ch2: SyncSender<i32>, m: Arc<Mutex<()>>) {{
            let _ = ch1.recv().unwrap();
            ch2.send(1).unwrap();
        }}
        fn main() {{
            let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
            let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);
            thread::spawn(move || s(ch1_tx, ch2_rx, Arc::new(Mutex::new(()))));
            thread::spawn(move || r(ch1_rx, ch2_tx, Arc::new(Mutex::new(()))));
        }}
        "
    );
    let doc = check(&src, &[("ch1", 0), ("ch2", 0)]);
    assert_eq!(attr(&doc, "main::ch1")["status"], "match");
    assert_eq!(attr(&doc, "main::ch1")["observed"], "rendezvous");
    assert_eq!(attr(&doc, "main::ch2")["status"], "match");
}

#[test]
fn sync_channel_one_conflicts_with_zero_and_matches_one() {
    let src = format!(
        "{HEADER}
        fn s(ch: SyncSender<i32>) {{ ch.send(1).unwrap(); }}
        fn main() {{
            let (tx, rx) = sync_channel::<i32>(1);
            thread::spawn(move || s(tx));
            let _ = rx.recv().unwrap();
        }}
        "
    );
    let zero = check(&src, &[("ch", 0)]);
    assert_eq!(attr(&zero, "main::ch")["status"], "mismatch");
    assert_eq!(attr(&zero, "main::ch")["observed"], "bounded");
    assert_eq!(attr(&zero, "main::ch")["observed_capacity"], 1);
    let one = check(&src, &[("ch", 1)]);
    assert_eq!(attr(&one, "main::ch")["status"], "match");
    let two = check(&src, &[("ch", 2)]);
    assert_eq!(attr(&two, "main::ch")["status"], "mismatch");
}

#[test]
fn non_literal_capacity_is_unknown() {
    let src = format!(
        "{HEADER}
        fn s(ch: SyncSender<i32>) {{ ch.send(1).unwrap(); }}
        fn main() {{
            let n = 1;
            let (tx, rx) = sync_channel::<i32>(n);
            thread::spawn(move || s(tx));
            let _ = rx.recv().unwrap();
        }}
        "
    );
    let doc = check(&src, &[("ch", 0)]);
    assert_eq!(attr(&doc, "main::ch")["status"], "unknown");
    assert_ne!(attr(&doc, "main::ch")["status"], "match");
}

#[test]
fn unused_channel_does_not_supply_capacity() {
    let src = format!(
        "{HEADER}
        fn s(ch1: Sender<i32>, ch2: Receiver<i32>) {{
            ch1.send(1).unwrap();
            let _ = ch2.recv().unwrap();
        }}
        fn r(ch1: Receiver<i32>, ch2: Sender<i32>) {{
            let _ = ch1.recv().unwrap();
            ch2.send(1).unwrap();
        }}
        fn main() {{
            let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
            let _ = (ch1_tx, ch1_rx);
            let (other_tx, other_rx) = channel::<i32>();
            let (back_tx, back_rx) = channel::<i32>();
            thread::spawn(move || s(other_tx, back_rx));
            thread::spawn(move || r(other_rx, back_tx));
        }}
        "
    );
    let doc = check(&src, &[("ch1", 0), ("ch2", 0)]);
    assert_eq!(attr(&doc, "main::ch1")["status"], "mismatch");
    assert_eq!(attr(&doc, "main::ch1")["observed"], "unbounded");
    assert_eq!(attr(&doc, "main::ch2")["observed"], "unbounded");
}

#[test]
fn renamed_endpoint_uses_the_traced_constructor() {
    let src = format!(
        "{HEADER}
        fn s(ch1: SyncSender<i32>, ch2: Receiver<i32>) {{
            ch1.send(1).unwrap();
            let _ = ch2.recv().unwrap();
        }}
        fn r(ch1: Receiver<i32>, ch2: SyncSender<i32>) {{
            let _ = ch1.recv().unwrap();
            ch2.send(1).unwrap();
        }}
        fn main() {{
            let (left, right) = sync_channel::<i32>(0);
            let (back_tx, back_rx) = sync_channel::<i32>(0);
            let (ch1_tx, ch1_rx) = channel::<i32>();
            let _ = (ch1_tx, ch1_rx);
            thread::spawn(move || s(left, back_rx));
            thread::spawn(move || r(right, back_tx));
        }}
        "
    );
    let doc = check(&src, &[("ch1", 0), ("ch2", 0)]);
    assert_eq!(attr(&doc, "main::ch1")["status"], "match");
    assert_eq!(attr(&doc, "main::ch1")["observed"], "rendezvous");
    assert_eq!(attr(&doc, "main::ch1")["evidence"], "param_from_call");
    assert_eq!(attr(&doc, "main::ch2")["status"], "match");
}

#[test]
fn renamed_use_is_not_paired_by_a_coincidental_name() {
    let src = format!(
        "{HEADER}
        fn s(port: SyncSender<i32>) {{ port.send(1).unwrap(); }}
        fn main() {{
            let (ch1_tx, ch1_rx) = channel::<i32>();
            let _ = (ch1_tx, ch1_rx);
            let (left, right) = sync_channel::<i32>(0);
            thread::spawn(move || s(left));
            let _ = right.recv().unwrap();
        }}
        "
    );
    let doc = check(&src, &[("ch1", 0)]);
    let verified = doc["verified"].as_object().unwrap();
    assert!(verified.values().all(|v| v["cir"] != "main::ch1" || v["rule"] != "channel-name"
        || verified.keys().any(|k| k.contains("port"))));
    assert!(doc["verified"].get("port").is_none());
    // The use-site name `port` has no channel token, so it is not verified as ch1
    // just because an unused variable is named ch1_tx.
    assert!(doc["verified"]
        .as_object()
        .unwrap()
        .iter()
        .all(|(k, v)| !(k.contains("port") && v["cir"] == "main::ch1")));
}

#[test]
fn assignment_replaces_the_previous_constructor() {
    let src = format!(
        "{HEADER}
        fn s(ch1: SyncSender<i32>) {{ ch1.send(1).unwrap(); }}
        fn main() {{
            let (mut ch1_tx, mut ch1_rx) = sync_channel::<i32>(0);
            let (other_tx, other_rx) = sync_channel::<i32>(1);
            ch1_tx = other_tx;
            ch1_rx = other_rx;
            thread::spawn(move || s(ch1_tx));
            let _ = ch1_rx.recv().unwrap();
        }}
        "
    );
    let doc = check(&src, &[("ch1", 0)]);
    assert_eq!(attr(&doc, "main::ch1")["status"], "mismatch");
    assert_eq!(attr(&doc, "main::ch1")["observed"], "bounded");
    assert_eq!(attr(&doc, "main::ch1")["observed_capacity"], 1);
}

#[test]
fn block_let_shadows_instead_of_keeping_the_old_constructor() {
    let src = format!(
        "{HEADER}
        fn s(ch1: SyncSender<i32>) {{ ch1.send(1).unwrap(); }}
        fn main() {{
            let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
            let (other_tx, other_rx) = sync_channel::<i32>(1);
            let ch1_tx = {{ other_tx }};
            let ch1_rx = {{ other_rx }};
            thread::spawn(move || s(ch1_tx));
            let _ = ch1_rx.recv().unwrap();
        }}
        "
    );
    let doc = check(&src, &[("ch1", 0)]);
    assert_eq!(attr(&doc, "main::ch1")["status"], "mismatch");
    assert_eq!(attr(&doc, "main::ch1")["observed"], "bounded");
    assert_eq!(attr(&doc, "main::ch1")["observed_capacity"], 1);
}

#[test]
fn unsupported_assignment_does_not_keep_the_old_constructor() {
    let src = format!(
        "{HEADER}
        fn s(ch1: SyncSender<i32>) {{ ch1.send(1).unwrap(); }}
        fn make() -> SyncSender<i32> {{ let (tx, _rx) = sync_channel::<i32>(0); tx }}
        fn main() {{
            let (mut ch1_tx, ch1_rx) = sync_channel::<i32>(0);
            ch1_tx = make();
            thread::spawn(move || s(ch1_tx));
            let _ = ch1_rx.recv().unwrap();
        }}
        "
    );
    let doc = check(&src, &[("ch1", 0)]);
    assert_eq!(attr(&doc, "main::ch1")["status"], "unknown");
}

#[test]
fn use_before_rebind_is_not_the_binding_after_rebind() {
    let src = format!(
        "{HEADER}
        fn main() {{
            let (mut ch1_tx, _rx) = sync_channel::<i32>(0);
            ch1_tx.send(1).unwrap();
            let (other_tx, other_rx) = sync_channel::<i32>(1);
            ch1_tx = other_tx;
            ch1_tx.send(2).unwrap();
            let _ = other_rx.recv().unwrap();
        }}
        "
    );
    let doc = check(&src, &[("ch1", 0)]);
    assert_eq!(attr(&doc, "main::ch1")["status"], "unknown");
}

#[test]
fn inner_scope_does_not_replace_the_outer_constructor() {
    let src = format!(
        "{HEADER}
        fn s(ch1: SyncSender<i32>) {{ ch1.send(1).unwrap(); }}
        fn main() {{
            let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
            {{
                let (ch1_tx, ch1_rx) = sync_channel::<i32>(1);
                let _ = (ch1_tx, ch1_rx);
            }}
            thread::spawn(move || s(ch1_tx));
            let _ = ch1_rx.recv().unwrap();
        }}
        "
    );
    let doc = check(&src, &[("ch1", 0)]);
    assert_eq!(attr(&doc, "main::ch1")["status"], "match");
    assert_eq!(attr(&doc, "main::ch1")["observed"], "rendezvous");
}

#[test]
fn barrier_is_reported_and_does_not_become_a_channel_match() {
    let src = format!(
        "{HEADER}
        fn s(ch1: SyncSender<i32>, ch2: Receiver<i32>, barrier: Arc<Barrier>) {{
            barrier.wait();
            ch1.send(1).unwrap();
            let _ = ch2.recv().unwrap();
        }}
        fn r(ch1: Receiver<i32>, ch2: SyncSender<i32>, barrier: Arc<Barrier>) {{
            barrier.wait();
            let _ = ch1.recv().unwrap();
            ch2.send(1).unwrap();
        }}
        fn main() {{
            let barrier = Arc::new(Barrier::new(2));
            let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
            let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);
            let barrier_s = Arc::clone(&barrier);
            let barrier_r = Arc::clone(&barrier);
            thread::spawn(move || s(ch1_tx, ch2_rx, barrier_s));
            thread::spawn(move || r(ch1_rx, ch2_tx, barrier_r));
        }}
        "
    );
    let doc = check(&src, &[("ch1", 0), ("ch2", 0)]);
    assert_eq!(attr(&doc, "main::ch1")["status"], "match");
    let gaps = doc["uncovered_sync"].as_array().unwrap();
    assert!(gaps.iter().any(|g| g["form"] == "Barrier::new"));
    assert!(gaps.iter().filter(|g| g["form"] == "wait").count() >= 2);
}

#[test]
fn cloned_sender_preserves_constructor_capacity_at_worker_use() {
    let src = format!("{HEADER}
        fn s(ch: SyncSender<i32>) {{ ch.send(1).unwrap(); }}
        fn r(ch: Receiver<i32>) {{ let _ = ch.recv().unwrap(); }}
        fn main() {{
            let (tx, rx) = sync_channel::<i32>(1);
            let tx_worker = tx.clone();
            thread::spawn(move || {{ s(tx_worker); }});
            thread::spawn(move || {{ r(rx); }});
        }}");
    let doc = check(&src, &[("ch", 1)]);
    assert_eq!(attr(&doc, "main::ch")["status"], "match", "{doc}");
    let wrong = check(&src, &[("ch", 0)]);
    assert_eq!(attr(&wrong, "main::ch")["status"], "mismatch", "{wrong}");
}

#[test]
fn cloned_shadowed_unknown_sender_does_not_recover_old_capacity() {
    let src = format!("{HEADER}
        fn unknown() -> SyncSender<i32> {{ sync_channel(2).0 }}
        fn s(ch: SyncSender<i32>) {{ ch.send(1).unwrap(); }}
        fn main() {{
            let (tx, rx) = sync_channel::<i32>(1);
            let tx = unknown();
            let tx_worker = tx.clone();
            thread::spawn(move || s(tx_worker));
        }}");
    let doc = check(&src, &[("ch", 1)]);
    assert_eq!(attr(&doc, "main::ch")["status"], "unknown", "{doc}");
}

#[test]
fn local_shadow_in_worker_closure_is_not_bound_to_captured_endpoint() {
    let src = format!("{HEADER}
        fn unknown() -> SyncSender<i32> {{ sync_channel(2).0 }}
        fn s(ch: SyncSender<i32>) {{ ch.send(1).unwrap(); }}
        fn main() {{
            let (tx, rx) = sync_channel::<i32>(1);
            thread::spawn(move || {{ let tx = unknown(); s(tx); }});
        }}");
    let doc = check(&src, &[("ch", 1)]);
    assert_eq!(attr(&doc, "main::ch")["status"], "unknown", "{doc}");
}
