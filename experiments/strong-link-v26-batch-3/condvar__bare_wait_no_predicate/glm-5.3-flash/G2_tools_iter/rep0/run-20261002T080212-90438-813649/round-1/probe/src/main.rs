use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // R2: one lock (m) and one condition variable (cv) guarding the boolean flag (ready).
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    // R1: waiter and notifier run at the same time.
    let waiter_handle = thread::spawn(move || {
        let (m, cv) = &*waiter_shared;

        // R4: check the flag first, wait only while the flag is false, re-check after each wake.
        // R6: the lock is released while blocked (Condvar::wait does this), and
        //     the waiter never waits when the flag is already true.
        // R9: the waiter does not pass its wait until the flag is true.
        let mut ready = m.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        // R5: if the notifier signals before the waiter starts waiting, the flag
        //     is already true here, so the waiter never waits and completes.
    });

    let notifier_handle = thread::spawn(move || {
        let (m, cv) = &*notifier_shared;

        // R3: set the flag to true while holding the lock, signal, then release the lock.
        let mut ready = m.lock().unwrap();
        *ready = true;
        cv.notify_one();
        drop(ready); // release the lock
    });

    // R7: joining both roles guarantees every interleaving terminates with both finished.
    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    // R8: the flag is true in every schedule by this point.
    // R10: print exactly the required line, then exit.
    println!("DONE ready=true");
}
