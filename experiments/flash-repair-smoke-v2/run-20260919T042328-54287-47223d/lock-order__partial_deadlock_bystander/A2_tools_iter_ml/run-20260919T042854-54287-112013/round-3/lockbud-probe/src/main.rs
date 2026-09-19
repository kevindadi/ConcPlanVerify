use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

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
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.cv.notify_one();
    }
}

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));
    let sem_a = Arc::new(Semaphore::new(0));
    let sem_b = Arc::new(Semaphore::new(0));

    // Worker A: signal readiness, wait for B's readiness, then lock a -> b.
    // No lock is held while waiting on a semaphore, so no cycle can form.
    let (ma, mb, sa, sb) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_a = thread::spawn(move || {
        sa.release();
        sb.acquire();
        let ga = ma.lock().unwrap();
        let gb = mb.lock().unwrap();
        drop(gb);
        drop(ga);
    });

    // Worker B: signal readiness, wait for A's readiness, then lock a -> b.
    let (ma2, mb2, sa2, sb2) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_b = thread::spawn(move || {
        sb2.release();
        sa2.acquire();
        let ga = ma2.lock().unwrap();
        let gb = mb2.lock().unwrap();
        drop(gb);
        drop(ga);
    });

    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10));
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
