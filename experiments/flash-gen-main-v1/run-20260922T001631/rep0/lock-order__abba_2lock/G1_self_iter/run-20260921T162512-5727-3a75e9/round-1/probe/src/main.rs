use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Two locks shared by both workers.
    let lock_a = Arc::new(Mutex::new(()));
    let lock_b = Arc::new(Mutex::new(()));

    // Counters to prove each worker ran exactly once.
    let t1_done = Arc::new(Mutex::new(0));
    let t2_done = Arc::new(Mutex::new(0));

    let mut handles = Vec::new();

    // Worker 1: acquire both locks in a consistent global order (A then B).
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

    // Worker 2: acquire both locks in the same global order (A then B).
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
}
