//! Fixed Rust for tests/e2e/partial_deadlock/buggy.json.
//! The original defect was a lock-order inversion: worker A took mtx_a then
//! mtx_b, while worker B took mtx_b then mtx_a. The semaphore handshake did
//! not prevent the cycle, so both workers could block forever even though the
//! detached bystander kept the process globally live.
//!
//! Fix: make both workers acquire the two mutexes in the same global order
//! (mtx_a before mtx_b). The semaphore handshake is preserved, and the
//! bystander still loops forever, so the system remains globally live while
//! every reachable state can still finish A and B.

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
        // Acquire in the same global order as worker A: mtx_a before mtx_b.
        // The handshake still forces B to wait for A's sem_a release before
        // it can proceed past the semaphore, but the lock order is now
        // consistent, so no cycle can form.
        sa2.acquire(); // s1: acquire sem_a (wait for A's handshake)
        let ga = ma2.lock().unwrap(); // s2: lock mtx_a
        sb2.release(); // s3: release sem_b
        let gb = mb2.lock().unwrap(); // s4: lock mtx_b
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
