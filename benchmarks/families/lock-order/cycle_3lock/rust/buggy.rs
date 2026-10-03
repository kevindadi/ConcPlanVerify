//! Reference Rust for tests/e2e/three_way_deadlock/buggy.json.
//! Expected defect: Deadlock (circular wait: w1 a->b, w2 b->c, w3 c->a).
//! Running this program can hang forever; it exists for Lockbud/Miri baselines.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));
    let mtx_c = Arc::new(Mutex::new(()));

    let (a1, b1) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w1 = thread::spawn(move || {
        let ga = a1.lock().unwrap(); // s1: lock mtx_a
        let gb = b1.lock().unwrap(); // s2: lock mtx_b
        drop(gb); // s3
        drop(ga); // s4
    });

    let (b2, c2) = (Arc::clone(&mtx_b), Arc::clone(&mtx_c));
    let w2 = thread::spawn(move || {
        let gb = b2.lock().unwrap(); // s1: lock mtx_b
        let gc = c2.lock().unwrap(); // s2: lock mtx_c
        drop(gc); // s3
        drop(gb); // s4
    });

    let (c3, a3) = (Arc::clone(&mtx_c), Arc::clone(&mtx_a));
    let w3 = thread::spawn(move || {
        let gc = c3.lock().unwrap(); // s1: lock mtx_c
        let ga = a3.lock().unwrap(); // s2: lock mtx_a (closes the cycle!)
        drop(ga); // s3
        drop(gc); // s4
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();
}
