//! Reference Rust for benchmarks/cir/goal_trivial/fixed.json.
//! Gold verdict: safe. The goal now demands that x reaches 2 in some
//! execution, which worker 2's write makes reachable and which does not
//! hold in the initial state.

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

    // Business goal g_x_reaches_two: reachable (w2 runs last in some schedule).
}
