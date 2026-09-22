use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// A counting semaphore built from a Mutex and a Condvar.
struct Semaphore {
    permits: Mutex<usize>,
    condvar: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Mutex::new(permits),
            condvar: Condvar::new(),
        }
    }

    /// Block until a permit is available, then take it.
    fn acquire(&self) {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            // Waiting releases the mutex, so the permit holder can
            // still lock it and release its permit.
            count = self.condvar.wait(count).unwrap();
        }
        *count -= 1;
    }

    /// Return a permit and wake one waiting worker.
    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        self.condvar.notify_one();
    }

    fn available(&self) -> usize {
        *self.permits.lock().unwrap()
    }
}

fn w1(s: Arc<Semaphore>) {
    s.acquire();
    // Critical section: work performed while holding the permit.
    let mut acc: u64 = 0;
    for i in 0..10_000u64 {
        acc = acc.wrapping_add(i);
    }
    std::hint::black_box(acc);
    s.release();
}

fn w2(s: Arc<Semaphore>) {
    s.acquire();
    // Critical section: work performed while holding the permit.
    let mut acc: u64 = 1;
    for i in 1..10_000u64 {
        acc = acc.wrapping_mul(i % 7 + 1);
    }
    std::hint::black_box(acc);
    s.release();
}

fn main() {
    // Shared counting permit pool with exactly one permit.
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let h1 = thread::spawn(move || w1(s1));

    let s2 = Arc::clone(&s);
    let h2 = thread::spawn(move || w2(s2));

    // Supervising task waits for both workers to finish.
    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE permits={}", s.available());
}
