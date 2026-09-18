//! Fixed Rust for tests/e2e/mutex_deadlock/buggy.json.
//! Defect was a classic lock-ordering deadlock: w1 acquired a then b, while
//! w2 acquired b then a. Both workers now acquire the mutexes in the same
//! global order (a then b), so no interleaving can deadlock.

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
        let ga = a2.lock().unwrap(); // s1: lock mtx_a (same order as w1)
        let gb = b2.lock().unwrap(); // s2: lock mtx_b
        drop(gb); // s3: drop mtx_b
        drop(ga); // s4: drop mtx_a
    });

    w1.join().unwrap();
    w2.join().unwrap();
}
