//! Reference Rust for benchmarks/cir/scale_branch_fan_4x2/buggy.json.
//! Gold verdict: safe. Four workers write their id to an unsynchronized-in-ConcIR
//! shared integer (modeled here with a mutex for Rust soundness), then walk
//! two chained branches on the observed value. Every branch arm terminates,
//! so all interleavings complete. Scaling case: ~29k CVN states.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let x = Arc::new(Mutex::new(0_i32));

    let handles: Vec<_> = (1..=4)
        .map(|id| {
            let x = Arc::clone(&x);
            thread::spawn(move || {
                *x.lock().unwrap() = id;
                for _ in 0..2 {
                    let seen = *x.lock().unwrap();
                    if seen == id {
                        // branch arm: still the last writer
                    } else {
                        // branch arm: someone else overwrote x
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}
