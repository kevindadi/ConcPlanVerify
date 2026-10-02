mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // R2: one lock (m), one condition variable (cv), guarding the boolean flag (ready).
    let shared = Arc::new((Mutex::new_named("shared_mutex0#191", false), Condvar::new_named("shared_condvar0#212")));

    // R1: waiter and notifier run at the same time.
    let waiter = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("waiter#344", move || {
            let (m, cv) = (&shared.0, &shared.1);

            // R4/R6: check the flag while holding the lock; wait only while it is false.
            let mut ready = m.lock().unwrap();
            while !*ready {
                // R6: wait() atomically releases m while blocked and reacquires it on wake.
                // R4/R9: re-check the flag after each wake; cannot pass until it is true.
                ready = cv.wait(ready).unwrap();
            }
            // Lock released here; flag is true.
        })
    };

    let notifier = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier#970", move || {
            let (m, cv) = (&shared.0, &shared.1);

            // R3: set the flag while holding the lock...
            let mut ready = m.lock().unwrap();
            *ready = true;
            // ...signal the condition variable...
            cv.notify_one();
            // ...then release the lock (guard dropped at end of scope).
        })
    };

    // R7: wait for both roles to finish in every schedule.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // R8/R10: the flag is true in every schedule; print exactly the required line.
    let shared_ready = shared.0.lock().unwrap();
    println!("DONE ready={}", *shared_ready);
 cir_trace::finish();}
