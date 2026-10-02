mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: m (lock), cv (condition variable), ready (shared variable).
    let shared = Arc::new((Mutex::new_named("shared_mutex0#187", false), Condvar::new_named("shared_condvar0#208")));
    let (m, cv) = (&shared.0, &shared.1);

    // Waiter role
    let waiter_handle = {
        let m = Arc::clone(&shared);
        cir_trace::spawn("waiter_handle#350", move || {
            let (lock, cv) = (&m.0, &m.1);
            let mut ready = lock.lock().unwrap();
            // Check the flag first; wait only while the flag is false.
            while !*ready {
                // Wait releases the lock while blocked and re-acquires it on wake.
                ready = cv.wait(ready).unwrap();
                // Re-check the flag after each wake (loop condition).
            }
            // Flag is true; the lock is released when `ready` goes out of scope.
        })
    };

    // Notifier role
    let notifier_handle = {
        let m = Arc::clone(&shared);
        cir_trace::spawn("notifier_handle#980", move || {
            let (lock, cv) = (&m.0, &m.1);
            let mut ready = lock.lock().unwrap();
            // Set the shared flag to true while holding the lock.
            *ready = true;
            // Signal the condition variable while holding the lock.
            cv.notify_one();
            // Lock is released when `ready` goes out of scope.
        })
    };

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    // Both roles finished; the flag is true in every schedule.
    let (lock, _cv) = (&shared.0, &shared.1);
    let ready = lock.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
