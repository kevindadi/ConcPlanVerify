use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // R2: one lock (m), one condition variable (cv), guarding the boolean flag (ready).
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    // R1: waiter role runs concurrently with the notifier role.
    let waiter = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            let (m, cv) = &*shared;
            // R4/R6: check the flag while holding the lock; wait only while false.
            // The guard is atomically released during wait and re-acquired on wake,
            // so the lock is never held while blocked.
            let mut ready = m.lock().expect("waiter: mutex poisoned");
            while !*ready {
                // R4/R9: re-check the flag after each wake (also covers spurious wakeups).
                ready = cv.wait(ready).expect("waiter: mutex poisoned");
            }
            // `ready` guard drops here, releasing m.
        })
    };

    let notifier = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            let (m, cv) = &*shared;
            // R3: set the flag while holding the lock, signal, then release the lock.
            let mut ready = m.lock().expect("notifier: mutex poisoned");
            *ready = true;
            cv.notify_one();
            drop(ready); // release m
        })
    };

    // R7: join both roles; every interleaving terminates because the waiter's
    // check-then-wait is atomic under m, so an early signal cannot be lost (R5).
    waiter.join().expect("waiter panicked");
    notifier.join().expect("notifier panicked");

    // R8/R10: the flag is true in every schedule; print the exact required line.
    let (m, _cv) = &*shared;
    let ready = *m.lock().expect("main: mutex poisoned");
    println!("DONE ready={}", ready);
}
