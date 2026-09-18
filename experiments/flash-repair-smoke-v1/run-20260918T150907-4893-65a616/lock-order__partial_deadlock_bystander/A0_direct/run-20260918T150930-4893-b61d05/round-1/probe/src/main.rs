//! Corrected Rust for tests/e2e/partial_deadlock/buggy.json.
//! The original program had a classic lock-order inversion between workers A
//! and B: A held mtx_a and wanted mtx_b, while B held mtx_b and wanted mtx_a.
//! The semaphore handshake did not prevent this, so both workers blocked
//! forever while the detached bystander kept the process globally live.
//!
//! Fix: enforce a global lock ordering. Both workers acquire mtx_a before
//! mtx_b. The semaphore handshake is preserved (each worker releases its
//! "ready" semaphore and waits on the other's), but the second lock is now
//! always taken in the same order, so no cycle can form. Both workers
//! complete, and the bystander still makes progress forever.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;

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
    let sem_a = Arc::new(Semaphore::new(0));
    let sem_b = Arc::new(Semaphore::new(0));

    let (ma, mb, sa, sb) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_a = thread::spawn(move || {
        let ga = ma.lock().unwrap(); // s1: lock mtx_a
        sa.release(); // s2: release sem_a
        sb.acquire(); // s3: acquire sem_b
        let gb = mb.lock().unwrap(); // s4: lock mtx_b (same order as B)
        drop(gb); // s5
        drop(ga); // s6
    });

    let (ma2, mb2, sa2, sb2) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_b = thread::spawn(move || {
        // Same global lock order as worker A: mtx_a before mtx_b.
        let ga = ma2.lock().unwrap(); // s1: lock mtx_a
        sb2.release(); // s2: release sem_b
        sa2.acquire(); // s3: acquire sem_a
        let gb = mb2.lock().unwrap(); // s4: lock mtx_b (same order as A)
        drop(gb); // s5
        drop(ga); // s6
    });

    // Detached bystander: loops forever, so the process as a whole keeps
    // making progress. It does not touch the workers' locks.
    thread::spawn(move || loop {
        std::hint::spin_loop();
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
