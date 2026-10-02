use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Shared resource: s (semaphore) — a counting permit pool.
struct Semaphore {
    permits: Mutex<u32>,
    available: Condvar,
}

impl Semaphore {
    fn new(initial: u32) -> Self {
        Semaphore {
            permits: Mutex::new(initial),
            available: Condvar::new(),
        }
    }

    // Acquire one permit. While waiting, the mutex lock is released,
    // so the worker currently holding a permit remains able to release it.
    fn acquire(&self) {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            count = self.available.wait(count).unwrap();
        }
        *count -= 1;
    }

    // Release one permit and wake a waiting worker (if any).
    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        self.available.notify_one();
    }
}

// Worker role: acquires one permit, performs its work, releases the permit.
fn worker(name: &'static str, s: Arc<Semaphore>) {
    s.acquire();
    // Critical section: hold the permit while performing the work,
    // so w1 and w2 never work at the same time.
    println!("{} working", name);
    s.release();
}

fn main() {
    // Supervising task: launches w1 and w2, waits for both to finish.
    // The permit pool begins with exactly one permit.
    let s = Arc::new(Semaphore::new(1));

    let w1 = {
        let s = Arc::clone(&s);
        thread::spawn(move || worker("w1", s))
    };
    let w2 = {
        let s = Arc::clone(&s);
        thread::spawn(move || worker("w2", s))
    };

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits=1");
}
