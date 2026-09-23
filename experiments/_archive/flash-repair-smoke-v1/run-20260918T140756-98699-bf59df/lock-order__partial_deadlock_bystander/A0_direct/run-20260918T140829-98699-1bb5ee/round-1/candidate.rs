//! Corrected Rust for tests/e2e/partial_deadlock/buggy.json.
//! The original defect: workers A and B pass the semaphore handshake, then A
//! holds mtx_a and wants mtx_b while B holds mtx_b and wants mtx_a. The
//! detached bystander keeps looping, so the system is not globally deadlocked,
//! but neither worker ever returns.
//!
//! Fix: enforce a global lock ordering. Both workers acquire mtx_a before
//! mtx_b, so the circular wait is broken. The semaphore handshake is kept
//! (it still coordinates the two workers), and the bystander still runs
//! forever, keeping the system globally live.

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
        // Acquire in the same global order (mtx_a before mtx_b) to avoid the
        // circular wait. The semaphore handshake is preserved.
        let ga = ma2.lock().unwrap(); // s1: lock mtx_a
        sb2.release(); // s2: release sem_b
        sa2.acquire(); // s3: acquire sem_a
        let gb = mb2.lock().unwrap(); // s4: lock mtx_b (same order as A)
        drop(gb); // s5
        drop(ga); // s6
    });

    // Detached bystander: loops forever, so the process as a whole keeps
    // making progress even while workers are contending.
    thread::spawn(move || loop {
        std::hint::spin_loop();
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
