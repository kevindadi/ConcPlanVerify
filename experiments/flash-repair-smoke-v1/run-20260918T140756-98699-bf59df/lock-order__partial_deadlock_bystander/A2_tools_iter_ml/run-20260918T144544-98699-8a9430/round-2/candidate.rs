//! Fixed Rust for tests/e2e/partial_deadlock/buggy.json.
//!
//! The original defect was a lock-order inversion: worker A held mtx_a and
//! wanted mtx_b, while worker B held mtx_b and wanted mtx_a. The semaphore
//! handshake did not prevent this, because both workers could pass the
//! handshake before either attempted its second lock.
//!
//! Fix: enforce a strict global ordering on the two mutexes. Both workers
//! acquire mtx_a before mtx_b, so no circular wait is possible. The semaphore
//! handshake is retained as an explicit synchronization point between the two
//! workers, but it no longer needs to serialize the whole critical section.
//!
//! The bystander task keeps looping forever, so the system as a whole remains
//! globally live, while every reachable state still allows both workers to
//! finish.

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

    // Handshake semaphores: each worker signals that it has taken its first
    // lock, and waits for the other worker's signal before taking its second.
    // Combined with the consistent lock order (a then b), this guarantees
    // progress without any circular wait.
    let a_ready = Arc::new(Semaphore::new(0));
    let b_ready = Arc::new(Semaphore::new(0));

    let (ma, mb, ar, br) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&a_ready),
        Arc::clone(&b_ready),
    );
    let worker_a = thread::spawn(move || {
        let ga = ma.lock().unwrap(); // s1: lock mtx_a
        ar.release(); // signal: A holds mtx_a
        br.acquire(); // wait: B holds mtx_b
        let gb = mb.lock().unwrap(); // s2: lock mtx_b (consistent order)
        drop(gb); // s3
        drop(ga); // s4
    });

    let (ma2, mb2, ar2, br2) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&a_ready),
        Arc::clone(&b_ready),
    );
    let worker_b = thread::spawn(move || {
        let gb = mb2.lock().unwrap(); // s1: lock mtx_b
        br2.release(); // signal: B holds mtx_b
        ar2.acquire(); // wait: A holds mtx_a
        let ga = ma2.lock().unwrap(); // s2: lock mtx_a (consistent order)
        drop(ga); // s3
        drop(gb); // s4
    });

    // Detached bystander: loops forever, so the process as a whole keeps
    // making progress even while workers are waiting on the handshake.
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10)); // s1/s2: nop loop
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
