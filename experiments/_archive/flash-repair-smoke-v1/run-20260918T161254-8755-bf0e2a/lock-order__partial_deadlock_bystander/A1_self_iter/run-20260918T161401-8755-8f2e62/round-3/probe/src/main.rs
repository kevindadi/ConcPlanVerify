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
        // Handshake first, without holding any lock, so the barrier can
        // never deadlock. Then take both mutexes in the global order
        // mtx_a -> mtx_b.
        sa.release(); // signal: A is ready
        sb.acquire(); // wait: B is ready
        let ga = ma.lock().unwrap(); // lock mtx_a
        let gb = mb.lock().unwrap(); // lock mtx_b
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
        // Symmetric handshake, then the same global lock order.
        sb2.release(); // signal: B is ready
        sa2.acquire(); // wait: A is ready
        let ga = ma2.lock().unwrap(); // lock mtx_a
        let gb = mb2.lock().unwrap(); // lock mtx_b
        drop(gb);
        drop(ga);
    });

    // Detached bystander: loops forever, so the process as a whole keeps
    // making progress even if a worker were stuck.
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10));
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
