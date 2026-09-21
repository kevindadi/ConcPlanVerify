mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Two locks shared by both workers.
    let lock_a = Arc::new(Mutex::new_named("lock_a_mutex0", ()));
    let lock_b = Arc::new(Mutex::new_named("lock_b_mutex0", ()));

    // Counters to prove each worker completed its critical work.
    let t1_done = Arc::new(Mutex::new_named("t1_done_mutex0", 0u32));
    let t2_done = Arc::new(Mutex::new_named("t2_done_mutex0", 0u32));

    let mut handles = Vec::new();

    // Worker 1 acquires lock_a then lock_b.
    {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let t1_done = Arc::clone(&t1_done);
        handles.push(thread::spawn(move || {
            let _ga = lock_a.lock().unwrap();
            let _gb = lock_b.lock().unwrap();
            // Critical work while holding both locks.
            *t1_done.lock().unwrap() = 1;
            // Locks released here when guards drop.
        }));
    }

    // Worker 2 acquires lock_a then lock_b (same order as worker 1 to avoid deadlock).
    {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let t2_done = Arc::clone(&t2_done);
        handles.push(thread::spawn(move || {
            let _ga = lock_a.lock().unwrap();
            let _gb = lock_b.lock().unwrap();
            // Critical work while holding both locks.
            *t2_done.lock().unwrap() = 1;
            // Locks released here when guards drop.
        }));
    }

    // Main thread waits for both workers to finish.
    for h in handles {
        h.join().unwrap();
    }

    let t1 = *t1_done.lock().unwrap();
    let t2 = *t2_done.lock().unwrap();
    println!("DONE t1={} t2={}", t1, t2);
 cir_trace::finish();}
