mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // R2: one lock (m) guarding the boolean flag (ready), and one condition variable (cv),
    // shared between the waiter and the notifier.
    let shared = Arc::new((Mutex::new_named("shared_mutex0#245", false), Condvar::new_named("shared_condvar0#266")));

    // R1: main task starts the waiter and the notifier, which run at the same time.
    let waiter_shared = Arc::clone(&shared);
    let waiter = cir_trace::spawn("waiter#423", move || {
        let (m, cv) = &*waiter_shared;

        // R6: acquire the lock before checking the flag.
        let mut ready = m.lock().unwrap();

        // R4: check the flag first; wait only while the flag is false; re-check after each wake.
        // R6: while blocked in cv.wait, the lock is released (Condvar::wait releases it).
        // R9: the loop only exits once the flag is true.
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        // Lock is released here when `ready` guard is dropped.
    });

    let notifier_shared = Arc::clone(&shared);
    let notifier = cir_trace::spawn("notifier#1054", move || {
        let (m, cv) = &*notifier_shared;

        // R3: set the shared flag to true while holding the lock, signal, then release.
        let mut ready = m.lock().unwrap();
        *ready = true;
        cv.notify_one();
        drop(ready); // release the lock
    });

    // R7: join both roles so every interleaving terminates with both finished.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // R5: even if the notifier signals before the waiter waits, the waiter's
    // `while !*ready` check sees the flag already true and never waits.

    // R8: the flag is true in every schedule (notifier always sets it under the lock).
    let (m, _) = &*shared;
    let ready = *m.lock().unwrap();

    // R10: print exactly `DONE ready=true` and exit.
    println!("DONE ready={}", ready);
 cir_trace::finish();}
