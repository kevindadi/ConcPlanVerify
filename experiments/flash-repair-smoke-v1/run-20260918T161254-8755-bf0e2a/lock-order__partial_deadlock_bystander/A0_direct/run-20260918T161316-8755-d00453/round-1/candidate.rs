//! Corrected Rust for tests/e2e/partial_deadlock/buggy.json.
//! The original program had a classic lock-order inversion: worker A held
//! mtx_a and waited for mtx_b, while worker B held mtx_b and waited for
//! mtx_a. The semaphore handshake did not prevent this because both workers
//! could pass the handshake before either attempted its second lock.
//!
//! Fix: enforce a global lock ordering. Both workers acquire mtx_a before
//! mtx_b. The semaphore handshake is preserved so that each worker still
//! signals its progress and waits for the other, but the second lock is
//! always taken in the same order, so no cycle can form. The bystander
//! remains an independent, forever-progressing task.

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
        // Worker B also takes mtx_a first, then mtx_b, matching worker A's
        // lock order. The handshake still forces B to wait for A's signal
        // before proceeding to the second lock.
        let ga = ma2.lock().unwrap(); // s1: lock mtx_a (same order as A)
        sb2.release(); // s2: release sem_b
        sa2.acquire(); // s3: acquire sem_a
        let gb = mb2.lock().unwrap(); // s4: lock mtx_b (same order as A)
        drop(gb); // s5
        drop(ga); // s6
    });

    // Detached bystander: loops forever, so the process as a whole keeps
    // making progress even while the workers synchronize.
    thread::spawn(move || loop {
        std::hint::spin_loop();
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
