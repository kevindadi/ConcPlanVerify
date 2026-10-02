// Supervised workers sharing a counting semaphore that starts with one permit.
// w1 and w2 each acquire the single permit, do their work, and release it,
// so they never work at the same time and every schedule terminates.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// A simple counting semaphore (permit pool).
struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            permits: Mutex::new(initial),
            available: Condvar::new(),
        }
    }

    /// Acquire one permit, blocking until one is available.
    /// The waiting worker holds no locks while blocked, so the worker
    /// holding the permit remains able to release it.
    fn acquire(&self) {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            count = self.available.wait(count).unwrap();
        }
        *count -= 1;
    }

    /// Release one previously acquired permit.
    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        self.available.notify_one();
    }

    /// Current number of available permits.
    fn permit_count(&self) -> usize {
        *self.permits.lock().unwrap()
    }
}

/// Worker body: acquire a permit, perform the work, release the permit.
fn worker(s: &Arc<Semaphore>) {
    s.acquire();

    // Perform the work while holding the permit.
    // (Deliberately silent so the program's only output is the final line.)
    let mut acc: u64 = 0;
    for i in 0..1000 {
        acc = acc.wrapping_add(i);
    }
    std::hint::black_box(acc);

    s.release();
}

fn main() {
    // Shared permit pool starting with exactly one permit.
    let s = Arc::new(Semaphore::new(1));

    // Supervising task: launch the two workers and wait for both to finish.
    let s1 = Arc::clone(&s);
    let h1 = thread::spawn(move || {
        let s = s1; // w1
        worker(&s);
    });

    let s2 = Arc::clone(&s);
    let h2 = thread::spawn(move || {
        let s = s2; // w2
        worker(&s);
    });

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // All permits released: the pool is back to its initial count.
    assert_eq!(s.permit_count(), 1);

    println!("DONE permits=1");
}
