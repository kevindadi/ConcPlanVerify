//! Fixed Rust for tests/e2e/partial_deadlock/buggy.json.
//! The original defect was a lock-order inversion: worker A held mtx_a and
//! wanted mtx_b, while worker B held mtx_b and wanted mtx_a. The semaphore
//! handshake did not prevent this, so both workers could block forever while
//! the detached bystander kept the process globally live.
//!
//! Fix: enforce a global lock ordering. Both workers acquire mtx_a before
//! mtx_b, so no cycle in the wait-for graph can form. The semaphore handshake
//! is preserved, and the bystander still loops forever.

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
        let gb = mb.lock().unwrap(); // s4: lock mtx_b (after mtx_a: consistent order)
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
        // Acquire in the same global order as worker A: mtx_a before mtx_b.
        // The semaphore handshake is preserved, but the lock acquisition
        // order no longer forms a cycle.
        sa2.acquire(); // s1: acquire sem_a (handshake)
        let ga = ma2.lock().unwrap(); // s2: lock mtx_a
        sb2.release(); // s3: release sem_b
        let gb = mb2.lock().unwrap(); // s4: lock mtx_b (after mtx_a)
        drop(gb); // s5
        drop(ga); // s6
    });

    // Detached bystander: loops forever, so the process as a whole keeps
    // making progress even while workers are running.
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10)); // s1/s2: nop loop
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
