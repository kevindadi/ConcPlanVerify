//! Reference Rust for benchmarks/cir/goal_bad_reference/fixed.json.
//! Gold verdict: safe. The goal now references the real control point
//! `w1.s4` (worker 1 reached its return) together with the declared shared
//! variable `x` at value 1, both witnessed by the schedule where w1 runs
//! to completion before w2 writes.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let x = Arc::new(Mutex::new(0_i32));

    let x1 = Arc::clone(&x);
    let w1 = thread::spawn(move || {
        *x1.lock().unwrap() = 1; // s1..s3: lock, write 1, drop
    });

    let x2 = Arc::clone(&x);
    let w2 = thread::spawn(move || {
        *x2.lock().unwrap() = 2; // s1..s3: lock, write 2, drop
    });

    w1.join().unwrap();
    w2.join().unwrap();

    // Business goal g_worker_done: w1 completed and x==1 held at that point.
}
