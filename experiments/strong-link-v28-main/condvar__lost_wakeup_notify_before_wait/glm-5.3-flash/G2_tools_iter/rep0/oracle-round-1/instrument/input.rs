use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // R2: one lock (m), one condition variable (cv), one boolean flag (ready)
    // guarded by the lock, shared between waiter and notifier.
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    // R1: waiter and notifier run at the same time as separate threads.
    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    let waiter_handle = thread::spawn(move || {
        let (m, cv) = &*waiter_shared;

        // R4: check the flag while holding the lock.
        let mut ready = m.lock().unwrap();

        // R4/R5: wait only while the flag is false; because the check and the
        // wait happen under the same lock hold, a signal that arrives before
        // this wait (with the flag already true) is never missed — the loop
        // simply never enters wait.
        while !*ready {
            // R7: cv.wait atomically releases the lock while blocked and
            // re-acquires it before returning.
            // R4: re-check the flag after every wake.
            ready = cv.wait(ready).unwrap();
        }
        // Lock is released when `ready` (the guard) goes out of scope.
    });

    let notifier_handle = thread::spawn(move || {
        let (m, cv) = &*notifier_shared;

        // R3: set the flag while holding the lock.
        let mut ready = m.lock().unwrap();
        *ready = true;

        // R6: the flag is true before the signal is issued, so a waiter that
        // is (or will be) checking under the lock never misses the notification.
        cv.notify_all();

        // R3: release the lock after signaling (guard drop at scope end).
        drop(ready);
    });

    // R8: join both roles; every interleaving terminates because the waiter
    // either sees the flag already true or is woken by the notifier's signal.
    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    // R9/R10: read the flag under the lock and print the required line.
    let (m, _cv) = &*shared;
    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
}
