use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // R2: one lock (m), one condition variable (cv), one boolean flag (ready) guarded by the lock.
    let shared = Arc::new((Mutex::new(false), Condvar::new()));
    let (m, cv) = (&shared.0, &shared.1);

    // Waiter role
    let waiter_shared = Arc::clone(&shared);
    let waiter = thread::spawn(move || {
        let (m, cv) = (&waiter_shared.0, &waiter_shared.1);
        // R4: check the flag while holding the lock; wait only while flag is false;
        // re-check after every wake. R7: lock is released while blocked.
        let mut ready = m.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        // R4: waiter completes once the flag is true.
        *ready
    });

    // Notifier role
    let notifier_shared = Arc::clone(&shared);
    let notifier = thread::spawn(move || {
        let (m, cv) = (&notifier_shared.0, &notifier_shared.1);
        // R3: set the flag to true while holding the lock, signal, then release.
        let mut ready = m.lock().unwrap();
        // R6: flag is set to true before the signal is issued.
        *ready = true;
        cv.notify_one();
        drop(ready); // release the lock
    });

    // R1: both roles run at the same time; R8: wait for both to finish.
    let waiter_result = waiter.join().unwrap();
    notifier.join().unwrap();

    // R9: the flag is true in every schedule; R5: waiter completes even if the
    // signal happened before it started waiting (flag check prevents missed wakeups).
    // R10: print exactly the required line and exit.
    println!("DONE ready={}", waiter_result);
}
