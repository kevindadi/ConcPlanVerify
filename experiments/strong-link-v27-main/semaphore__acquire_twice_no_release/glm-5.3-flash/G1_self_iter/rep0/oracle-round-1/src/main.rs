mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared resource: s — a counting permit pool (semaphore) starting with 1 permit.
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

    // Block until a permit is available, then take it.
    fn acquire(&self) {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            // wait() atomically releases the mutex while blocked, so a
            // permit holder can still lock the mutex and release (R5).
            count = self.available.wait(count).unwrap();
        }
        *count -= 1;
    }

    // Return a permit and wake one waiter.
    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        drop(count); // unlock before notifying: woken thread never blocks on a held lock
        self.available.notify_one();
    }

    fn count(&self) -> usize {
        *self.permits.lock().unwrap()
    }
}

// Worker role: acquires the permit, does its work, releases it.
// Structured acquire/release with no early return, so every
// acquisition path is matched by exactly one release (R4).
fn worker(name: &'static str, s: Arc<Semaphore>, rounds: usize) {
    for round in 0..rounds {
        s.acquire();
        // Critical section: only one worker can be here at a time (R3).
        println!("{} working round {}", name, round);
        s.release();
    }
}

fn main() { cir_trace::init();
    // Supervising task: launches w1 and w2, waits for both (R1).
    let s = Arc::new(Semaphore::new(1)); // exactly one permit initially (R2)

    let mut handles = Vec::new();
    for name in ["w1", "w2"] {
        let s = Arc::clone(&s);
        handles.push(thread::spawn(move || worker(name, s, 2)));
    }

    for h in handles {
        h.join().unwrap();
    }

    // All acquires were matched by releases, so the pool is back to 1.
    let done = s.count();
    println!("DONE done={}", done); // prints exactly: DONE done=1 (R7)
 cir_trace::finish();}
