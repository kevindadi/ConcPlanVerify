//! Reference Rust for benchmarks/cir/goal_unreachable/fixed.json.
//! Fix: worker 1 writes the required value 3, so the business goal
//! "x reaches 3" is satisfiable (when w1 runs after w2, x ends at 3).

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let x = Arc::new(Mutex::new(0_i32));

    let x1 = Arc::clone(&x);
    let w1 = thread::spawn(move || {
        *x1.lock().unwrap() = 3; // s1..s3: lock, write 3, drop
    });

    let x2 = Arc::clone(&x);
    let w2 = thread::spawn(move || {
        *x2.lock().unwrap() = 2; // s1..s3: lock, write 2, drop
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let final_x = *x.lock().unwrap();
    assert!(final_x == 2 || final_x == 3);
}
