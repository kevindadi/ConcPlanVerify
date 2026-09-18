//! Reference Rust for tests/e2e/dual_condvar/buggy.json.
//! Expected defect: SignalLoss / mutual sleep. Thread A waits on cv1 before
//! anyone can notify it, and thread B waits on cv2 symmetrically; each
//! thread's notify comes only *after* its own wait, so both sleep forever.
//! Running this program can hang forever; it exists for Lockbud/Miri baselines.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let pair1 = Arc::new((Mutex::new(()), Condvar::new())); // m1 + cv1
    let pair2 = Arc::new((Mutex::new(()), Condvar::new())); // m2 + cv2

    let (p1a, p2a) = (Arc::clone(&pair1), Arc::clone(&pair2));
    let thread_a = thread::spawn(move || {
        let (m1, cv1) = &*p1a;
        let (m2, cv2) = &*p2a;
        let g1 = m1.lock().unwrap(); // s1: lock m1
        let g1 = cv1.wait(g1).unwrap(); // s2: wait cv1 (blocks forever)
        let g2 = m2.lock().unwrap(); // s3: lock m2
        cv2.notify_all(); // s4: notify cv2 — never reached
        drop(g2); // s5
        drop(g1); // s6
    });

    let (p1b, p2b) = (Arc::clone(&pair1), Arc::clone(&pair2));
    let thread_b = thread::spawn(move || {
        let (m1, cv1) = &*p1b;
        let (m2, cv2) = &*p2b;
        let g2 = m2.lock().unwrap(); // s1: lock m2
        let g2 = cv2.wait(g2).unwrap(); // s2: wait cv2 (blocks forever)
        let g1 = m1.lock().unwrap(); // s3: lock m1
        cv1.notify_all(); // s4: notify cv1 — never reached
        drop(g1); // s5
        drop(g2); // s6
    });

    thread_a.join().unwrap();
    thread_b.join().unwrap();
}
