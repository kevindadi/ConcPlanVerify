use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // R2: one lock (m) and one condition variable (cv) guarding the boolean flag (ready)
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    // R1: main task starts the waiter and the notifier, which run at the same time
    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    let waiter = thread::spawn(move || {
        let (m, cv) = &*waiter_shared;

        // R4: check the flag first, while holding the lock
        let mut ready = m.lock().unwrap();

        // R4/R6/R9: wait only while the flag is false; re-check after each wake;
        // the lock is released while blocked and reacquired on wake
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        // R9: only pass the wait once the flag is true
    });

    let notifier = thread::spawn(move || {
        let (m, cv) = &*notifier_shared;

        // R3: set the flag to true while holding the lock, signal, then release
        let mut ready = m.lock().unwrap();
        *ready = true;
        cv.notify_one();
        drop(ready); // release the lock
    });

    // R7: join both roles so every interleaving terminates with both finished
    waiter.join().unwrap();
    notifier.join().unwrap();

    // R8/R10: the flag is true in every schedule; print exactly the required line
    let (m, _) = &*shared;
    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
}
