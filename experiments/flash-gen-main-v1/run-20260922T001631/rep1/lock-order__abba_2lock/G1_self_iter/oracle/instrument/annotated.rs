mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Two locks shared by both workers.
    let lock_a = Arc::new(Mutex::new_named("lock_a_mutex0", ()));
    let lock_b = Arc::new(Mutex::new_named("lock_b_mutex0", ()));

    // Track completion of each worker.
    let done1 = Arc::new(Mutex::new_named("done1_mutex0", 0));
    let done2 = Arc::new(Mutex::new_named("done2_mutex0", 0));

    let (a1, b1, d1) = (Arc::clone(&lock_a), Arc::clone(&lock_b), Arc::clone(&done1));
    let (a2, b2, d2) = (Arc::clone(&lock_a), Arc::clone(&lock_b), Arc::clone(&done2));

    // Worker 1 acquires locks in order A then B.
    let t1 = cir_trace::spawn("t1", move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        // Critical work while holding both locks.
        *d1.lock().unwrap() = 1;
        // Locks released here (drop order: _gb then _ga).
    });

    // Worker 2 acquires locks in the same order A then B to avoid deadlock.
    let t2 = cir_trace::spawn("t2", move || {
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        // Critical work while holding both locks.
        *d2.lock().unwrap() = 1;
        // Locks released here.
    });

    // Main thread waits for both workers to finish.
    t1.join().unwrap();
    t2.join().unwrap();

    let r1 = *done1.lock().unwrap();
    let r2 = *done2.lock().unwrap();
    println!("DONE t1={} t2={}", r1, r2);
 cir_trace::finish();}
