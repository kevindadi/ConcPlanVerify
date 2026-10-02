mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let shared = Arc::new((Mutex::new_named("shared_mutex0#102", false), Condvar::new_named("shared_condvar0#123")));

    let waiter = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("waiter#202", move || {
            let (m, cv) = (&shared.0, &shared.1);
            // R4/R6: check the flag first; wait only while it is false.
            let mut ready = m.lock().unwrap();
            while !*ready {
                ready = cv.wait(ready).unwrap();
                // Re-check the flag after each wake (loop condition).
            }
            *ready
        })
    };

    let notifier = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier#667", move || {
            let (m, cv) = (&shared.0, &shared.1);
            // R3: set flag while holding the lock, signal, then release.
            let mut ready = m.lock().unwrap();
            *ready = true;
            cv.notify_one();
            drop(ready);
        })
    };

    let ready = waiter.join().unwrap();
    notifier.join().unwrap();
    // R10
    println!("DONE ready={}", ready);
 cir_trace::finish();}
