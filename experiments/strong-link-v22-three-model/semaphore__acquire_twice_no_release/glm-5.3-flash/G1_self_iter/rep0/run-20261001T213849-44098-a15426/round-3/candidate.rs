use std::sync::{Arc, Condvar, Mutex};
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

    // Blocking acquire. Condvar::wait releases the mutex while parked, so a
    // worker holding the permit is never blocked by a waiter (R5).
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

// Role: w1 / w2 — straight-line acquire/work/release pairs; each acquire is
// unconditionally paired with a release, so acquire count == release count
// on every path (R3, R4).
fn worker(name: &'static str, s: Arc<Semaphore>) {
    for round in 0..2 {
        s.acquire();
        let _ = (name, round); // work under the permit; nothing blocking here (R5, R6)
        s.release();
    }
}

// Role: supervising task — launches both workers and joins them (R1).
fn main() {
    // The pool begins with exactly one permit (R2).
    let s = Arc::new(Semaphore::new(1));

    let s_for_w1 = Arc::clone(&s);
    let w1 = thread::spawn(move || worker("w1", s_for_w1));

    let s_for_w2 = Arc::clone(&s);
    let w2 = thread::spawn(move || worker("w2", s_for_w2));

    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // All interleavings terminate (R6); exactly one output line (R7).
    println!("DONE done=1");
}
