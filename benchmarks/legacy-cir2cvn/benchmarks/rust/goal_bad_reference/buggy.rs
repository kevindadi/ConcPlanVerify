//! Reference Rust for benchmarks/cir/goal_bad_reference/buggy.json.
//! Gold verdict: goals unmet (goal references nonexistent entities). The
//! declared goal points at a marking key `w1_done` that is neither a
//! resource nor a control-place reference, and a variable `y` that is not
//! declared. The code itself is behaviorally correct — the defect is a
//! dangling specification that must be rejected instead of silently passed.

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

    // Business goal g_worker_done: refers to `w1_done` / `y`, neither exists.
}
