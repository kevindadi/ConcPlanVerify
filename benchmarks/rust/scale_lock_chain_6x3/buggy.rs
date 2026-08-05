//! Reference Rust for benchmarks/cir/scale_lock_chain_6x3/buggy.json.
//! Gold verdict: safe. Six workers each acquire the three mutexes in the
//! same global order (m1 -> m2 -> m3) and release in reverse, so no
//! circular wait is possible. Scaling case: ~53k CVN states.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let locks: Vec<Arc<Mutex<()>>> = (0..3).map(|_| Arc::new(Mutex::new(()))).collect();

    let handles: Vec<_> = (0..6)
        .map(|_| {
            let locks = locks.clone();
            thread::spawn(move || {
                let _g1 = locks[0].lock().unwrap();
                let _g2 = locks[1].lock().unwrap();
                let _g3 = locks[2].lock().unwrap();
                // guards drop in reverse declaration order: m3, m2, m1
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}
