use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // R2: one lock (m), one condition variable (cv), one boolean flag (ready)
    // guarded by the lock, shared between the two roles.
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    // R1: waiter and notifier run concurrently as spawned threads.
    let waiter = thread::spawn(move || {
        let (m, cv) = &*waiter_shared;
        // R4/R7: check the flag while holding the lock.
        let mut ready = m.lock().unwrap();
        // R4: wait only while the flag is false; re-check after every wake
        // (the while loop handles spurious wakeups and lost-signal races).
        while !*ready {
            // R7: the lock is released while blocked and reacquired on wake.
            ready = cv.wait(ready).unwrap();
        }
    });

    let notifier = thread::spawn(move || {
        let (m, cv) = &*notifier_shared;
        // R3/R6/R7: set the flag to true while holding the lock, then signal.
        let mut ready = m.lock().unwrap();
        *ready = true;
        cv.notify_one();
        // R3: lock is released when `ready` is dropped at scope end.
    });

    // R8: join both roles so every interleaving terminates with both finished.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // R7: read the flag only while the lock is held.
    let (m, _) = &*shared;
    let ready = m.lock().unwrap();

    // R10: print exactly one line and exit.
    println!("DONE ready={}", *ready);
}
