//! Fixed Rust for tests/e2e/partial_deadlock/buggy.json.
//! Defect: workers A and B pass the semaphore handshake, then A holds mtx_a
//! and wants mtx_b while B holds mtx_b and wants mtx_a — a classic lock-order
//! inversion (partial deadlock). The detached bystander keeps looping, so the
//! system is not globally deadlocked, but neither worker ever returns.
//!
//! Fix: the handshake requires the two workers to take *different* first locks
//! (otherwise one blocks the other before it can signal), which inherently
//! makes each worker's second lock the other's first lock. To break the
//! inversion, after the handshake each worker releases its first lock and then
//! both re-acquire the two mutexes in the same global order (mtx_a before
//! mtx_b). The semaphore handshake is preserved: each worker still releases its
//! own semaphore after taking its first lock and waits on the other's semaphore
//! before proceeding. With a consistent lock order, the second acquisition can
//! never deadlock against the other worker, so both workers always complete.

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
        sb.acquire(); // s3: acquire sem_b (handshake)
        drop(ga); // release first lock before re-acquiring in global order
        let ga = ma.lock().unwrap(); // s4: lock mtx_a (global order: a then b)
        let gb = mb.lock().unwrap(); // s5: lock mtx_b
        drop(gb); // s6
        drop(ga); // s7
    });

    let (ma2, mb2, sa2, sb2) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_b = thread::spawn(move || {
        let gb = mb2.lock().unwrap(); // s1: lock mtx_b
        sb2.release(); // s2: release sem_b
        sa2.acquire(); // s3: acquire sem_a (handshake)
        drop(gb); // release first lock before re-acquiring in global order
        let ga = ma2.lock().unwrap(); // s4: lock mtx_a (global order: a then b)
        let gb = mb2.lock().unwrap(); // s5: lock mtx_b
        drop(gb); // s6
        drop(ga); // s7
    });

    // Detached bystander: loops forever, so the process as a whole keeps
    // making progress even while both workers are stuck.
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10)); // s1/s2: nop loop
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
