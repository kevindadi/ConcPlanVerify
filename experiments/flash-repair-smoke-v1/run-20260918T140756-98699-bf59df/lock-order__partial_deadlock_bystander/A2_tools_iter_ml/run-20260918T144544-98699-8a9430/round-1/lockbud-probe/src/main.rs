//! Fixed Rust for tests/e2e/partial_deadlock/buggy.json.
//! The original defect was a lock-order inversion: worker A held mtx_a and
//! wanted mtx_b, while worker B held mtx_b and wanted mtx_a. The semaphore
//! handshake did not prevent this, because both workers could pass the
//! handshake before either attempted its second lock.
//!
//! Fix: make the semaphore handshake enforce a strict ordering so that the
//! two workers cannot simultaneously hold their first lock and wait for the
//! second. Worker A acquires mtx_a, then waits for permission from B before
//! taking mtx_b; worker B acquires mtx_b, then waits for permission from A
//! before taking mtx_a. By having each worker release its "I have my first
//! lock" permit only after it has finished with both locks, we ensure that
//! at most one worker is ever in the critical section that requires both
//! mutexes, eliminating the circular wait.
//!
//! Concretely, we use a single "turn" semaphore initialized to 1. A worker
//! must acquire the turn before taking its first lock and releases it only
//! after dropping both locks. This serializes the two-lock critical sections
//! while still allowing the bystander to make progress forever.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

/// Minimal counting semaphore (std has none).
struct Semaphore {
    permits: Mutex<u32>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: u32) -> Self {
        Self {
            permits: Mutex::new(count),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.cv.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        *self.permits.lock().unwrap() += 1;
        self.cv.notify_one();
    }
}

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));
    // Single turnstile semaphore: only one worker may hold both mutexes at a
    // time, which removes the circular wait entirely.
    let turn = Arc::new(Semaphore::new(1));

    let (ma, mb, t) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b), Arc::clone(&turn));
    let worker_a = thread::spawn(move || {
        t.acquire(); // enter the two-lock critical section
        let ga = ma.lock().unwrap(); // s1: lock mtx_a
        let gb = mb.lock().unwrap(); // s2: lock mtx_b
        drop(gb); // s3
        drop(ga); // s4
        t.release(); // leave the critical section
    });

    let (ma2, mb2, t2) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b), Arc::clone(&turn));
    let worker_b = thread::spawn(move || {
        t2.acquire(); // enter the two-lock critical section
        let gb = mb2.lock().unwrap(); // s1: lock mtx_b
        let ga = ma2.lock().unwrap(); // s2: lock mtx_a
        drop(ga); // s3
        drop(gb); // s4
        t2.release(); // leave the critical section
    });

    // Detached bystander: loops forever, so the process as a whole keeps
    // making progress even while workers are waiting on the turnstile.
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10)); // s1/s2: nop loop
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
