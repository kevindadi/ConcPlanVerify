//! Reference Rust for benchmarks/cir/scale_lock_chain_5x3_buggy/buggy.json.
//! Gold verdict: deadlock. Workers 1-4 acquire m1 -> m2 -> m3, but worker 5
//! acquires m3 -> m2 -> m1, creating a circular wait. Scaling case:
//! ~14k CVN states explored before the deadlock is found.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let locks: Vec<Arc<Mutex<()>>> = (0..3).map(|_| Arc::new(Mutex::new(()))).collect();

    let mut handles = Vec::new();
    for _ in 0..4 {
        let locks = locks.clone();
        handles.push(thread::spawn(move || {
            let _g1 = locks[0].lock().unwrap();
            let _g2 = locks[1].lock().unwrap();
            let _g3 = locks[2].lock().unwrap();
        }));
    }

    // Worker 5 takes the locks in reverse order: circular wait with the rest.
    let locks5 = locks.clone();
    handles.push(thread::spawn(move || {
        let _g3 = locks5[2].lock().unwrap();
        let _g2 = locks5[1].lock().unwrap();
        let _g1 = locks5[0].lock().unwrap();
    }));

    for h in handles {
        h.join().unwrap();
    }
}
