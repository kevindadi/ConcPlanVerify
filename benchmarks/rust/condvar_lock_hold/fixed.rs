//! Reference Rust for tests/e2e/condvar_lock_hold/fixed.json.
//! Fix: drop `outer` before waiting, and re-check `ready` in a loop so a
//! notification that arrives early is not lost.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let outer = Arc::new(Mutex::new(()));
    let pair = Arc::new((Mutex::new(false), Condvar::new()));

    let (outer_w, pair_w) = (Arc::clone(&outer), Arc::clone(&pair));
    let waiter = thread::spawn(move || {
        let g_outer = outer_w.lock().unwrap(); // s1: lock outer
        let (inner, cv) = &*pair_w;
        let mut ready = inner.lock().unwrap(); // s2: lock inner
        drop(g_outer); // s3: release outer before wait
        while !*ready {
            // s4/s5: predicate loop
            ready = cv.wait(ready).unwrap();
        }
        drop(ready); // s6
    });

    let (outer_n, pair_n) = (Arc::clone(&outer), Arc::clone(&pair));
    let notifier = thread::spawn(move || {
        let g_outer = outer_n.lock().unwrap(); // s1
        let (inner, cv) = &*pair_n;
        let mut ready = inner.lock().unwrap(); // s2
        *ready = true; // s3
        cv.notify_all(); // s4
        drop(ready); // s5
        drop(g_outer); // s6
    });

    waiter.join().unwrap();
    notifier.join().unwrap();
}
