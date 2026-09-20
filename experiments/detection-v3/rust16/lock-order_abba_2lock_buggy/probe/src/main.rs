//! Reference Rust for tests/e2e/mutex_deadlock/buggy.json.
//! Expected defect: Deadlock (w1 locks a->b, w2 locks b->a).
//! Running this program can hang forever; it exists for Lockbud/Miri baselines.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));

    let (a1, b1) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w1 = thread::spawn(move || {
        let ga = a1.lock().unwrap(); // s1: lock mtx_a
        let gb = b1.lock().unwrap(); // s2: lock mtx_b
        drop(gb); // s3: drop mtx_b
        drop(ga); // s4: drop mtx_a
    });

    let (a2, b2) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w2 = thread::spawn(move || {
        let gb = b2.lock().unwrap(); // s1: lock mtx_b (opposite order!)
        let ga = a2.lock().unwrap(); // s2: lock mtx_a
        drop(ga); // s3: drop mtx_a
        drop(gb); // s4: drop mtx_b
    });

    w1.join().unwrap();
    w2.join().unwrap();
}
