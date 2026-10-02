mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // R2: one lock (m) and one condition variable (cv) guarding the shared flag (ready)
    let shared = Arc::new((Mutex::new_named("shared_mutex0#191", false), Condvar::new_named("shared_condvar0#212")));

    // R1: spawn the waiter and the notifier so they run at the same time
    let waiter_shared = Arc::clone(&shared);
    let waiter = cir_trace::spawn("waiter#358", move || {
        let (m, cv) = &*waiter_shared;

        // R4: check the flag first while holding the lock
        let mut guard = m.lock().unwrap();

        // R4/R6: wait only while the flag is false; re-check after each wake
        while !*guard {
            // R6: cv.wait releases the lock while blocked and reacquires it on wake
            guard = cv.wait(guard).unwrap();
        }

        // R9: only pass the wait once the flag is true
        *guard
    });

    let notifier_shared = Arc::clone(&shared);
    let notifier = cir_trace::spawn("notifier#914", move || {
        let (m, cv) = &*notifier_shared;

        // R3: set the flag to true while holding the lock
        let mut guard = m.lock().unwrap();
        *guard = true;

        // R3: signal the condition variable, then release the lock
        cv.notify_one();
        drop(guard);
    });

    // R5: even if the notifier signals before the waiter waits, the waiter's
    // initial flag check (under the lock) sees ready == true and never waits.

    let ready = waiter.join().unwrap();
    notifier.join().unwrap();

    // R7/R8: both roles are finished and the flag is true in every schedule.
    // R10: print exactly this line, then exit.
    println!("DONE ready={}", ready);
 cir_trace::finish();}
