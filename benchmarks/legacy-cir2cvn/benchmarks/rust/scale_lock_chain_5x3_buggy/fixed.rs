//! Reference Rust for benchmarks/cir/scale_lock_chain_5x3_buggy/fixed.json.
//! Fix: all five workers acquire the mutexes in the same global order.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let locks: Vec<Arc<Mutex<()>>> = (0..3).map(|_| Arc::new(Mutex::new(()))).collect();

    let handles: Vec<_> = (0..5)
        .map(|_| {
            let locks = locks.clone();
            thread::spawn(move || {
                let _g1 = locks[0].lock().unwrap();
                let _g2 = locks[1].lock().unwrap();
                let _g3 = locks[2].lock().unwrap();
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}
