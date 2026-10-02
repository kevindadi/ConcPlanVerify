mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared resource: s (semaphore) — a counting semaphore initialized with exactly one permit.
struct Semaphore {
    count: Mutex<i64>,
    available: Condvar,
}

impl Semaphore {
    fn new(n: i64) -> Self {
        Semaphore { count: Mutex::new(n), available: Condvar::new() }
    }

    // Blocking acquire. The waiter does NOT hold the semaphore's internal
    // lock while waiting (Condvar::wait releases the mutex), so the worker
    // currently holding the permit remains able to release it (R5).
    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count <= 0 {
            count = self.available.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.available.notify_one();
    }
}

// Role: w1 / w2 — a worker that acquires the permit, performs work, and
// releases it. Each worker acquires exactly twice (more than once, R4),
// and each acquire is immediately paired with a release on the same
// straight-line path with no early return or panic in between, so the
// permit is always released exactly as many times as it was acquired.
fn worker(name: &'static str, s: Arc<Semaphore>) {
    for round in 0..2 {
        s.acquire();
        // --- begin work under permit ---
        // Note: while holding the permit we perform no blocking waits,
        // so we can never deadlock the holder (R5, R6).
        let _ = (name, round);
        // --- end work under permit ---
        s.release();
    }
}

// Role: supervising task — launches both workers and joins them (R1).
fn main() { cir_trace::init();
    // The pool begins with exactly one permit (R2).
    let s = Arc::new(Semaphore::new(1));

    let s_for_w1 = Arc::clone(&s);
    let w1 = cir_trace::spawn("worker#1863", move || worker("w1", s_for_w1));

    let s_for_w2 = Arc::clone(&s);
    let w2 = cir_trace::spawn("worker#1959", move || worker("w2", s_for_w2));

    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // Every interleaving terminates (R6): each acquire waits only on the
    // condvar (never while holding the permit), each paired release
    // restores the permit, so the other worker eventually proceeds.
    // Exactly one line of output, then exit (R7).
    println!("DONE done=1");
 cir_trace::finish();}
