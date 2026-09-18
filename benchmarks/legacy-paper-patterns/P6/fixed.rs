//! Reference Rust for tests/e2e/dual_condvar/fixed.json.
//! Fix: the mutual condvar handshake is removed entirely; both threads just
//! take the two mutexes in the same global order (m1 then m2).

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));

    let (m1a, m2a) = (Arc::clone(&m1), Arc::clone(&m2));
    let thread_a = thread::spawn(move || {
        let g1 = m1a.lock().unwrap(); // s1: lock m1
        let g2 = m2a.lock().unwrap(); // s2: lock m2
        drop(g2); // s3
        drop(g1); // s4
    });

    let (m1b, m2b) = (Arc::clone(&m1), Arc::clone(&m2));
    let thread_b = thread::spawn(move || {
        let g1 = m1b.lock().unwrap(); // s1: lock m1 (same order)
        let g2 = m2b.lock().unwrap(); // s2: lock m2
        drop(g2); // s3
        drop(g1); // s4
    });

    thread_a.join().unwrap();
    thread_b.join().unwrap();
}
