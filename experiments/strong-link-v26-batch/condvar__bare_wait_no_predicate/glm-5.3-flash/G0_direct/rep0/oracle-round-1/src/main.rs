mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: m (lock), cv (condition variable), ready (shared variable).
    let shared = Arc::new((Mutex::new_named("shared_mutex0#187", false), Condvar::new_named("shared_condvar0#208")));
    let (m, cv) = (&shared.0, &shared.1);

    // Waiter role.
    let waiter = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("waiter#349", move || {
            let (m, cv) = (&shared.0, &shared.1);
            let mut ready = m.lock().unwrap();
            // Check the flag first; wait only while the flag is false;
            // re-check after each wake. The lock is released while blocked.
            while !*ready {
                ready = cv.wait(ready).unwrap();
            }
            // Flag is true: the waiter may pass its wait.
        })
    };

    // Notifier role.
    let notifier = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier#881", move || {
            let (m, cv) = (&shared.0, &shared.1);
            let mut ready = m.lock().unwrap();
            // Set the flag while holding the lock, then signal.
            *ready = true;
            cv.notify_one();
            // Lock is released when the guard is dropped.
        })
    };

    waiter.join().unwrap();
    notifier.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
