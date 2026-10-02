mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // R2: one lock (m), one condition variable (cv), one boolean flag (ready) guarded by m.
    let shared = Arc::new((Mutex::new_named("shared_mutex0#195", false), Condvar::new_named("shared_condvar0#216")));

    let waiter_shared = Arc::clone(&shared);
    let waiter = cir_trace::spawn("waiter#288", move || {
        let (m, cv) = &*waiter_shared;
        // R4, R7: check the flag while holding the lock; wait only while it is false.
        // R4: re-check after every wake (while-loop guards against spurious wakeups).
        // R7: wait atomically releases m while blocked and reacquires it on wake.
        let mut ready = m.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        // `ready` (the MutexGuard) is dropped here, releasing the lock.
    });

    let notifier_shared = Arc::clone(&shared);
    let notifier = cir_trace::spawn("notifier#878", move || {
        let (m, cv) = &*notifier_shared;
        // R3, R6, R7: set the flag to true while holding the lock,
        // then signal, then release the lock.
        let mut ready = m.lock().unwrap();
        *ready = true;
        cv.notify_one();
        drop(ready); // release the lock
    });

    // R1: waiter and notifier run concurrently; join both so every schedule terminates (R8).
    waiter.join().unwrap();
    notifier.join().unwrap();

    // R9, R10: the flag is true in every schedule; print exactly this line and exit.
    let (m, _cv) = &*shared;
    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
