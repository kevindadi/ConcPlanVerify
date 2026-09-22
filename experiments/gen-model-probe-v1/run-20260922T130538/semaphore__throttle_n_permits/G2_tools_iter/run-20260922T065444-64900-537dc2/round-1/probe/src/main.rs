use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut guard = self.permits.lock().unwrap();
        while *guard == 0 {
            guard = self.available.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    fn release(&self) {
        let mut guard = self.permits.lock().unwrap();
        *guard += 1;
        drop(guard);
        self.available.notify_one();
    }
}

fn main() {
    // Supervising task: create the shared pool with exactly two permits,
    // launch three workers, and wait for all of them to finish.
    let pool = Arc::new(Semaphore::new(2));

    let mut workers = Vec::new();
    for _ in 0..3 {
        let pool = Arc::clone(&pool);
        workers.push(thread::spawn(move || {
            pool.acquire();
            // Perform work while holding one permit.
            pool.release();
        }));
    }

    for worker in workers {
        worker.join().unwrap();
    }

    println!("DONE done=1");
}
