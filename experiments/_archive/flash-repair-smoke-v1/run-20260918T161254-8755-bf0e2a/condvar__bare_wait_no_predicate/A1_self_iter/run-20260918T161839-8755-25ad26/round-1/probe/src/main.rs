//! Reference Rust for tests/e2e/signal_loss/buggy.json.
//! Fixed defect: SignalLoss (lost wakeup). The waiter now checks the
//! predicate in a loop, so a notification that arrives before the waiter
//! waits is not lost and the waiter terminates.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // `ready` is the protected flag; the Mutex is `mtx` in the ConcIR.
    let pair = Arc::new((Mutex::new(false), Condvar::new()));

    let pair_w = Arc::clone(&pair);
    let waiter = thread::spawn(move || {
        let (mtx, cv) = &*pair_w;
        let mut guard = mtx.lock().unwrap(); // s1: lock mtx
        // s2: wait until `ready` becomes true, re-checking the predicate
        // after every wakeup to avoid the lost-wakeup bug.
        while !*guard {
            guard = cv.wait(guard).unwrap();
        }
        drop(guard); // s4: drop mtx
    });

    let pair_n = Arc::clone(&pair);
    let notifier = thread::spawn(move || {
        let (mtx, cv) = &*pair_n;
        let mut ready = mtx.lock().unwrap(); // s1: lock mtx
        *ready = true; // s2: write ready = true
        cv.notify_all(); // s3: notify_all
        drop(ready); // s4: drop mtx
    });

    waiter.join().unwrap();
    notifier.join().unwrap();
}
