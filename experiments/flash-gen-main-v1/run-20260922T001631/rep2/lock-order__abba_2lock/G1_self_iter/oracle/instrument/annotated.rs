mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Two locks shared by both workers.
    let lock_a = Arc::new(Mutex::new_named("lock_a_mutex0", ()));
    let lock_b = Arc::new(Mutex::new_named("lock_b_mutex0", ()));

    // Counters to prove each worker ran exactly once.
    let t1_done = Arc::new(Mutex::new_named("t1_done_mutex0", 0));
    let t2_done = Arc::new(Mutex::new_named("t2_done_mutex0", 0));

    let a1 = Arc::clone(&lock_a);
    let b1 = Arc::clone(&lock_b);
    let c1 = Arc::clone(&t1_done);

    let a2 = Arc::clone(&lock_a);
    let b2 = Arc::clone(&lock_b);
    let c2 = Arc::clone(&t2_done);

    // Worker 1 acquires locks in order A then B.
    let h1 = cir_trace::spawn("h1", move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        // Critical work while holding both locks.
        *c1.lock().unwrap() += 1;
        // Locks released here when guards drop.
    });

    // Worker 2 acquires locks in the same order A then B to avoid deadlock.
    let h2 = cir_trace::spawn("h2", move || {
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        // Critical work while holding both locks.
        *c2.lock().unwrap() += 1;
        // Locks released here when guards drop.
    });

    // Main thread waits for both workers to finish.
    h1.join().unwrap();
    h2.join().unwrap();

    let v1 = *t1_done.lock().unwrap();
    let v2 = *t2_done.lock().unwrap();
    println!("DONE t1={} t2={}", v1, v2);
 cir_trace::finish();}
