//! Dense twin of goal_constrained_deadlock: four branching workers; w3's
//! else-arm reverses m1/m2 and alone writes result=99 (business goal).

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));
    let flag = Arc::new(Mutex::new(0_i32));
    let result = Arc::new(Mutex::new(0_i32));

    let mut handles = Vec::new();
    for wid in [1_i32, 2, 4] {
        let (m1, m2, flag, result) = (
            Arc::clone(&m1), Arc::clone(&m2), Arc::clone(&flag), Arc::clone(&result),
        );
        handles.push(thread::spawn(move || {
            *flag.lock().unwrap() = wid;
            let mine = *flag.lock().unwrap() == wid;
            let _g1 = m1.lock().unwrap();
            let _g2 = m2.lock().unwrap();
            *result.lock().unwrap() = if mine { wid } else { wid + 10 };
        }));
    }

    let (m1, m2, flag, result) = (
        Arc::clone(&m1), Arc::clone(&m2), Arc::clone(&flag), Arc::clone(&result),
    );
    handles.push(thread::spawn(move || {
        *flag.lock().unwrap() = 3;
        if *flag.lock().unwrap() == 3 {
            let _g1 = m1.lock().unwrap();
            *result.lock().unwrap() = 3;
        } else {
            let _g2 = m2.lock().unwrap();
            let _g1 = m1.lock().unwrap();
            *result.lock().unwrap() = 99;
        }
    }));

    for h in handles {
        h.join().unwrap();
    }
}
