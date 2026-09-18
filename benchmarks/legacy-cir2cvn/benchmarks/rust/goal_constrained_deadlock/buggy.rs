//! Reference Rust for goal_constrained_deadlock/buggy.json.
//! Deadlock: w3's else-arm takes m2 before m1 while w1/w2 take m1 before m2.
//! Business goal (asserted for dynamic baselines): result can equal 99 —
//! written only on that else-arm. A "normalize all writers" rewrite that
//! drops the 99 write would pass a deadlock check but fail the goal.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));
    let flag = Arc::new(Mutex::new(0_i32));
    let result = Arc::new(Mutex::new(0_i32));

    let mut handles = Vec::new();
    for wid in 1..=2_i32 {
        let (m1, m2, flag, result) = (
            Arc::clone(&m1),
            Arc::clone(&m2),
            Arc::clone(&flag),
            Arc::clone(&result),
        );
        handles.push(thread::spawn(move || {
            *flag.lock().unwrap() = wid;
            let _g1 = m1.lock().unwrap();
            let _g2 = m2.lock().unwrap();
            *result.lock().unwrap() = wid;
        }));
    }

    let (m1, m2, flag, result) = (
        Arc::clone(&m1),
        Arc::clone(&m2),
        Arc::clone(&flag),
        Arc::clone(&result),
    );
    handles.push(thread::spawn(move || {
        *flag.lock().unwrap() = 3;
        let mine = *flag.lock().unwrap() == 3;
        if mine {
            let _g1 = m1.lock().unwrap();
            *result.lock().unwrap() = 3;
        } else {
            // Buried bug: m2 before m1, but also the only writer of 99.
            let _g2 = m2.lock().unwrap();
            let _g1 = m1.lock().unwrap();
            *result.lock().unwrap() = 99;
        }
    }));

    for h in handles {
        h.join().unwrap();
    }
}
