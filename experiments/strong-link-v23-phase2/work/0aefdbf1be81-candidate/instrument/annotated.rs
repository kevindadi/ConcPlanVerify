mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared resource: s (semaphore) — a counting permit pool (R2).
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

    // Acquire one permit. While waiting, the mutex is released, so the
    // worker currently holding a permit remains able to release it (R5).
    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.available.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    // Release one permit and wake a waiting worker so it can proceed (R5, R6).
    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        drop(permits);
        self.available.notify_one();
    }

    fn permits(&self) -> usize {
        *self.permits.lock().unwrap()
    }
}

// Common worker body: acquire, hold the permit while working (R3),
// then release. There is no early return or panic between acquire and
// release, so every path releases exactly as many times as it acquired (R4).
fn worker_body(s: &Semaphore, rounds: u32) {
    for _ in 0..rounds {
        s.acquire();
        // Critical section: only one worker can be here at a time (R3).
        for _ in 0..4 {
            std::hint::spin_loop();
            std::thread::yield_now();
        }
        s.release();
    }
}

// Role: w1 — acquires the permit twice, releasing after each use (R4).
fn w1(s: Arc<Semaphore>) {
    worker_body(&s, 2);
}

// Role: w2 — acquires the permit twice, releasing after each use (R4).
fn w2(s: Arc<Semaphore>) {
    worker_body(&s, 2);
}

fn main() { cir_trace::init();
    // One counting permit pool starting with exactly one permit (R2).
    let s = Arc::new(Semaphore::new(1));

    let s_for_w1 = Arc::clone(&s);
    let s_for_w2 = Arc::clone(&s);

    // Supervising task launches both workers (R1).
    let h1 = cir_trace::spawn("w1#2091", move || w1(s_for_w1));
    let h2 = cir_trace::spawn("w2#2141", move || w2(s_for_w2));

    // Supervisor waits for both workers to finish (R1, R6).
    h1.join().unwrap();
    h2.join().unwrap();

    // All permits returned: the pool is back to its initial count of 1.
    // Print exactly the required line and exit (R7).
    println!("DONE done={}", s.permits());
 cir_trace::finish();}
