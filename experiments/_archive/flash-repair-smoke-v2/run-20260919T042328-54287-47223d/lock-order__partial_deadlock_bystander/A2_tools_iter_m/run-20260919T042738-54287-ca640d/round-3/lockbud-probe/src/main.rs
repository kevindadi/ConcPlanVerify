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
    // sem_a starts with a permit: it is A's turn first.
    let sem_a = Arc::new(Semaphore::new(1));
    let sem_b = Arc::new(Semaphore::new(0));

    let (ma, mb, sa, sb) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_a = thread::spawn(move || {
        // Wait for our turn before touching any mutex.
        sa.acquire();
        // Consistent global lock order: mtx_a then mtx_b.
        let ga = ma.lock().unwrap();
        let gb = mb.lock().unwrap();
        // Hand the turn to B only after we hold both locks, so B cannot
        // begin acquiring locks until A is finished with them.
        sb.release();
        drop(gb);
        drop(ga);
    });

    let (ma2, mb2, sa2, sb2) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_b = thread::spawn(move || {
        // Wait for our turn (signaled by A) before touching any mutex.
        sb2.acquire();
        // Same global lock order as A: mtx_a then mtx_b.
        let ga = ma2.lock().unwrap();
        let gb = mb2.lock().unwrap();
        // Hand the turn back to A.
        sa2.release();
        drop(gb);
        drop(ga);
    });

    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10));
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
