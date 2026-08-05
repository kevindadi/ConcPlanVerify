//! Reference Rust for tests/e2e/semaphore_throttle/buggy.json.
//! Gold verdict: SAFE (negative control). Three workers throttled by a
//! two-permit semaphore; every acquire is eventually satisfied because
//! permits are always released. Baselines must NOT report a defect here.

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
    let sem = Arc::new(Semaphore::new(2));

    let handles: Vec<_> = (0..3)
        .map(|_| {
            let sem = Arc::clone(&sem);
            thread::spawn(move || {
                sem.acquire(); // s1: acquire one permit
                sem.release(); // s2: release the permit
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
