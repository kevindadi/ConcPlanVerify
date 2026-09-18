//! Reference Rust for benchmarks/cir/goal_unreachable/buggy.json.
//! Gold verdict: goals unmet. Workers write 1 and 2 to the shared value, so
//! the business requirement "x reaches 3" never holds in any interleaving.
//! The final assert encodes that requirement for dynamic baselines.

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

    // Business goal g_x_equals_three: never satisfied (x is 1 or 2).
    assert_eq!(*x.lock().unwrap(), 3, "goal g_x_equals_three unmet");
}
