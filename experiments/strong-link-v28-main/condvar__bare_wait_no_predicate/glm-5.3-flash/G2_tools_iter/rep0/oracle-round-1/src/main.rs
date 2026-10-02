mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: m (lock), cv (condition variable), ready (shared flag)
    let shared = Arc::new((Mutex::new_named("shared_mutex0#182", false), Condvar::new_named("shared_condvar0#203")));

    // Waiter role: checks the flag while holding the lock, waits only while
    // the flag is false, and re-checks the flag after each wake.
    let waiter = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("waiter#425", move || {
            let (m, cv) = &*shared;
            let mut ready = m.lock().unwrap();
            while !*ready {
                // Blocks without holding the lock; re-acquires it on wake.
                ready = cv.wait(ready).unwrap();
            }
            // Flag is true; waiter completes.
        })
    };

    // Notifier role: sets the flag to true while holding the lock, signals
    // the condition variable, then releases the lock.
    let notifier = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier#967", move || {
            let (m, cv) = &*shared;
            let mut ready = m.lock().unwrap();
            *ready = true;
            cv.notify_one();
            // Lock released when `ready` guard drops.
        })
    };

    // Both roles run concurrently; joining both guarantees every
    // interleaving terminates with both roles finished.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // The flag is true in every schedule (notifier always sets it).
    let ready = *shared.0.lock().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
