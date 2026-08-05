//! Reference Rust for tests/e2e/condvar_lock_hold/buggy.json.
//! Expected defect: condvar-induced deadlock. The waiter calls wait while
//! still holding `outer`; the notifier must acquire `outer` before it can
//! set `ready` and notify, so the waiter can strand the notifier forever.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let outer = Arc::new(Mutex::new(()));
    let pair = Arc::new((Mutex::new(false), Condvar::new())); // inner + cv

    let (outer_w, pair_w) = (Arc::clone(&outer), Arc::clone(&pair));
    let waiter = thread::spawn(move || {
        let g_outer = outer_w.lock().unwrap(); // s1: lock outer — held across wait
        let (inner, cv) = &*pair_w;
        let g_inner = inner.lock().unwrap(); // s2: lock inner
        let g_inner = cv.wait(g_inner).unwrap(); // s3: wait releases inner only
        drop(g_inner); // s4
        drop(g_outer); // s5 — never reached on the deadlock schedule
    });

    let (outer_n, pair_n) = (Arc::clone(&outer), Arc::clone(&pair));
    let notifier = thread::spawn(move || {
        let g_outer = outer_n.lock().unwrap(); // s1: lock outer
        let (inner, cv) = &*pair_n;
        let mut ready = inner.lock().unwrap(); // s2: lock inner
        *ready = true; // s3
        cv.notify_all(); // s4
        drop(ready); // s5
        drop(g_outer); // s6
    });

    waiter.join().unwrap();
    notifier.join().unwrap();
}
