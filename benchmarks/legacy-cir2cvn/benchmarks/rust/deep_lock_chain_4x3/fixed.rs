//! Reference Rust for benchmarks/cir/deep_lock_chain_4x3/fixed.json.
//! Fix: every arm of every worker follows the global order m1 -> m2 -> m3.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));
    let m3 = Arc::new(Mutex::new(()));
    let flag = Arc::new(Mutex::new(0_i32));

    let mut handles = Vec::new();
    for wid in 1..=4_i32 {
        let (m1, m2, m3, flag) = (
            Arc::clone(&m1),
            Arc::clone(&m2),
            Arc::clone(&m3),
            Arc::clone(&flag),
        );
        handles.push(thread::spawn(move || {
            *flag.lock().unwrap() = wid;
            let _mine = *flag.lock().unwrap() == wid;
            let _g1 = m1.lock().unwrap();
            let _g2 = m2.lock().unwrap();
            let _g3 = m3.lock().unwrap();
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}
