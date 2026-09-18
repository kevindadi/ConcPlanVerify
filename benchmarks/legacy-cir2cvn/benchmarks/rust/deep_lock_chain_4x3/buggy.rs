//! Reference Rust for benchmarks/cir/deep_lock_chain_4x3/buggy.json.
//! Gold verdict: deadlock, deeply buried. Four workers branch on a shared
//! flag; every arm follows the global order m1 -> m2 -> m3 except worker 3's
//! else-arm, which takes m2 before m1. That single hoisted acquisition
//! conflicts with every other worker's m1-before-m2 order — a circular wait
//! reachable only through that arm under contention.

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
            let mine = *flag.lock().unwrap() == wid;
            if wid == 3 {
                if mine {
                    // fast path: skips m2 entirely
                    let _g1 = m1.lock().unwrap();
                    let _g3 = m3.lock().unwrap();
                } else {
                    // slow path: m2 hoisted in front of m1 (the buried bug)
                    let _g2 = m2.lock().unwrap();
                    let _g1 = m1.lock().unwrap();
                    let _g3 = m3.lock().unwrap();
                }
            } else if mine {
                let _g1 = m1.lock().unwrap();
                let _g2 = m2.lock().unwrap();
                let _g3 = m3.lock().unwrap();
            } else {
                let _g1 = m1.lock().unwrap();
                let _g2 = m2.lock().unwrap();
                let _g3 = m3.lock().unwrap();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}
