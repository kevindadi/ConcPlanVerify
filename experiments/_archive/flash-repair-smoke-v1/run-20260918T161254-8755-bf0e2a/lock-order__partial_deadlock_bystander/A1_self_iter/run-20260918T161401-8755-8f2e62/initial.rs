//! Reference Rust for tests/e2e/partial_deadlock/buggy.json.
//! Expected defect: goals unmet (partial deadlock). Workers A and B pass the
//! semaphore handshake, then A holds mtx_a and wants mtx_b while B holds
//! mtx_b and wants mtx_a. The detached bystander keeps looping, so the
//! system is not globally deadlocked — but neither worker ever returns.
//! Running this program hangs on the joins; it exists for Lockbud/Miri baselines.

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
        let gb = mb.lock().unwrap(); // s4: lock mtx_b — deadlocks against worker_b
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
        let gb = mb2.lock().unwrap(); // s1: lock mtx_b
        sb2.release(); // s2: release sem_b
        sa2.acquire(); // s3: acquire sem_a
        let ga = ma2.lock().unwrap(); // s4: lock mtx_a — deadlocks against worker_a
        drop(ga); // s5
        drop(gb); // s6
    });

    // Detached bystander: loops forever, so the process as a whole keeps
    // making progress even while both workers are stuck.
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10)); // s1/s2: nop loop
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
