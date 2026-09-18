//! Reference Rust for benchmarks/cir/goal_trivial/buggy.json.
//! Gold verdict: goals unmet (goal too weak). The declared goal "x can be
//! observed at 0" already holds in the initial state, so it constrains
//! nothing: the program would pass the goal even if both workers were
//! deleted. Dynamic and static baselines cannot see this defect at all —
//! the code below is behaviorally correct; the flaw lives in the spec.

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

    // Business goal g_x_zero: trivially true before any thread runs.
}
