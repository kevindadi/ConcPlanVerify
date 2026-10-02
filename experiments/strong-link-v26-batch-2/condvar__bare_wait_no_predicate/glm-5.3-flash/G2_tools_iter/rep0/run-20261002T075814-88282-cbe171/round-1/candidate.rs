use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // R2: one lock (m) and one condition variable (cv) guarding a boolean flag (ready)
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    // --- waiter role ---
    let waiter_shared = Arc::clone(&shared);
    let waiter = thread::spawn(move || {
        let (m, cv) = &*waiter_shared;
        // R6: lock the mutex to inspect the flag
        let mut ready = m.lock().unwrap();
        // R4: check the flag first; wait only while the flag is false
        while !*ready {
            // R6: wait releases the lock while blocked and reacquires it on wake
            ready = cv.wait(ready).unwrap();
            // R4: re-check the flag after each wake (loop condition does this)
        }
        // R9: the loop only exits when ready is true
        assert!(*ready);
        // R10: print the required line
        println!("DONE ready={}", *ready);
    });

    // --- notifier role ---
    let notifier_shared = Arc::clone(&shared);
    let notifier = thread::spawn(move || {
        let (m, cv) = &*notifier_shared;
        // R3: set the flag to true while holding the lock
        let mut ready = m.lock().unwrap();
        *ready = true;
        // R3: signal the condition variable while holding the lock
        cv.notify_one();
        // R3: releasing the lock happens when `ready` guard is dropped at scope end
        drop(ready);
    });

    // R1: both roles run concurrently; R7: joining both guarantees termination
    waiter.join().unwrap();
    notifier.join().unwrap();
}
