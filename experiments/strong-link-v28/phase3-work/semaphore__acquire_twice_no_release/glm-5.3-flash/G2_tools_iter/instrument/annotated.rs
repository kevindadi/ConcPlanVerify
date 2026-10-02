mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Counting permit pool (semaphore) shared by w1 and w2.
// Implemented with a Mutex + Condvar so that a worker waiting for a permit
// releases the internal lock while waiting, leaving the permit holder free
// to release (R5).
struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            permits: Mutex::new(initial),
            cv: Condvar::new(),
        }
    }

    // Block until a permit is available, then take it.
    fn acquire(&self) {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            // wait() atomically releases the lock, so the holder can release.
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }

    // Give back one permit and wake a waiting worker (if any).
    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        self.cv.notify_one();
    }
}

// Worker body: acquires the permit, performs its work while holding it (R3),
// then releases it. Each worker may go through this cycle more than once (R4),
// and every path releases exactly as many times as it acquired.
fn worker(name: &'static str, s: Arc<Semaphore>) {
    for round in 0..2 {
        s.acquire();
        // --- critical section: only one worker here at a time (R3) ---
        println!("{} working round {}", name, round);
        // --- end of critical section ---
        s.release();
    }
}

fn main() { cir_trace::init();
    // Supervising task: launches w1 and w2 and waits for both (R1).
    // The shared permit pool starts with exactly one permit (R2).
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let h1 = cir_trace::spawn("worker#1828", move || worker("w1", s1));
    let h2 = cir_trace::spawn("worker#1882", move || worker("w2", s2));

    // Joining both workers guarantees every schedule terminates (R6):
    // waiting workers never hold the internal lock, so the permit holder
    // can always release and let the other proceed.
    h1.join().unwrap();
    h2.join().unwrap();

    // Exact required output (R7).
    println!("DONE done=1");
 cir_trace::finish();}
