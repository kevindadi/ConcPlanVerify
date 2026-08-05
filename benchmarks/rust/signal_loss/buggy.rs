//! Reference Rust for tests/e2e/signal_loss/buggy.json.
//! Expected defect: SignalLoss (lost wakeup). The waiter calls wait()
//! unconditionally, so if the notifier runs first the notification is lost
//! and the waiter blocks forever.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // `ready` is the protected flag; the Mutex is `mtx` in the CIR.
    let pair = Arc::new((Mutex::new(false), Condvar::new()));

    let pair_w = Arc::clone(&pair);
    let waiter = thread::spawn(move || {
        let (mtx, cv) = &*pair_w;
        let guard = mtx.lock().unwrap(); // s1: lock mtx
        // s2: wait without checking `ready` first — the lost-wakeup bug.
        let guard = cv.wait(guard).unwrap();
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
