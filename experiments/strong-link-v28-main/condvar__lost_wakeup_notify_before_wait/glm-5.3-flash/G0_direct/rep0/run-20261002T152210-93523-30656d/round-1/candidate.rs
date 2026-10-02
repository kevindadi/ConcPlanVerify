use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // R2: one lock (m), one condition variable (cv), and one boolean flag (ready)
    // guarded by the lock, shared between the waiter and the notifier.
    let shared = Arc::new((Mutex::new(false), Condvar::new()));
    let (m, cv) = (&shared.0, &shared.1);

    // Waiter role
    let waiter_shared = Arc::clone(&shared);
    let waiter = thread::spawn(move || {
        let (m, cv) = (&waiter_shared.0, &waiter_shared.1);
        // R4: check the flag while holding the lock; wait only while the flag
        // is false; re-check the flag after every wake.
        let mut ready = m.lock().unwrap();
        while !*ready {
            // R7: the lock is released while blocked, reacquired on wake.
            ready = cv.wait(ready).unwrap();
        }
        // R5: if the notifier signaled before the waiter started waiting,
        // the flag is already true here, so the loop is skipped and the
        // waiter completes without waiting.
    });

    // Notifier role
    let notifier_shared = Arc::clone(&shared);
    let notifier = thread::spawn(move || {
        let (m, cv) = (&notifier_shared.0, &notifier_shared.1);
        // R3: set the flag to true while holding the lock, signal the
        // condition variable, then release the lock (drop at end of scope).
        let mut ready = m.lock().unwrap();
        // R6: the flag is set to true before the signal is issued, so a
        // waiting role never misses the notification.
        *ready = true;
        cv.notify_one();
    });

    // R1: both roles run at the same time; R8: join both so every
    // interleaving terminates with the waiter and the notifier finished.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // R9: the flag is true in every schedule.
    let ready = *m.lock().unwrap();
    // R10: print exactly this line and exit.
    println!("DONE ready={}", ready);
}
