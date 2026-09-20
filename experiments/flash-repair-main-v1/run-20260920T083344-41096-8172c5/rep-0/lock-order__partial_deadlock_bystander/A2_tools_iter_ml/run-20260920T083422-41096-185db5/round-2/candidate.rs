use std::sync::{Arc, Condvar, Mutex};
use std::thread;

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

    // Worker A: lock mtx_a, signal sem_a, wait on sem_b, then lock mtx_b.
    let (ma, mb, sa, sb) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    let worker_a = thread::spawn(move || {
        let ga = ma.lock().unwrap();
        sa.release();
        sb.acquire();
        let gb = mb.lock().unwrap();
        drop(gb);
        drop(ga);
    });

    // Worker B: signal sem_b, wait on sem_a, then lock mtx_a and mtx_b
    // (same lock order as A, and it never holds mtx_a while waiting on sem_a).
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

    // Independent third task that keeps making progress.
    let _done = thread::spawn(move || loop {
        thread::yield_now();
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();

    println!("DONE a=1 b=1");

    std::process::exit(0);
}
