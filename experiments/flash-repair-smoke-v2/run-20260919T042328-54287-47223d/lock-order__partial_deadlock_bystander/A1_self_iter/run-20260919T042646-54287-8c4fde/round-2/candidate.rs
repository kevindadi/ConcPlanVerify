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
        *self.permits.lock().unwrap() += 1;
        self.cv.notify_one();
    }
}

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));
    // Turnstile: only one worker may be inside its critical section at a time,
    // which prevents the circular wait between mtx_a and mtx_b.
    let turnstile = Arc::new(Semaphore::new(1));
    let sem_a = Arc::new(Semaphore::new(0));
    let sem_b = Arc::new(Semaphore::new(0));

    let (ma, mb, sa, sb, t) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
        Arc::clone(&turnstile),
    );
    let worker_a = thread::spawn(move || {
        // Rendezvous first: release our semaphore, then wait for the peer's.
        // Both releases happen before either wait, so neither worker can
        // block the other's handshake.
        sa.release();
        sb.acquire();
        // Now serialize the mutex acquisition with the turnstile.
        t.acquire();
        let ga = ma.lock().unwrap();
        let gb = mb.lock().unwrap();
        drop(gb);
        drop(ga);
        t.release();
    });

    let (ma2, mb2, sa2, sb2, t2) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
        Arc::clone(&turnstile),
    );
    let worker_b = thread::spawn(move || {
        sb2.release();
        sa2.acquire();
        t2.acquire();
        let gb = mb2.lock().unwrap();
        let ga = ma2.lock().unwrap();
        drop(ga);
        drop(gb);
        t2.release();
    });

    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(10));
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();
}
