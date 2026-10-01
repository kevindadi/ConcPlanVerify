use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Shared resource: s (semaphore) — a counting semaphore initialized with exactly one permit (R2).
struct Semaphore {
    count: Mutex<i64>,
    available: Condvar,
}

impl Semaphore {
    fn new(n: i64) -> Self {
        Semaphore { count: Mutex::new(n), available: Condvar::new() }
    }

    // Blocking acquire. While parked, the mutex is released, so the current
    // permit holder is never blocked from releasing (R5).
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
// unconditionally paired with exactly one release, on every path (R3, R4).
fn worker(name: &'static str, s: Arc<Semaphore>) {
    for round in 0..2 {
        s.acquire();
        let _ = (name, round); // work under the permit; nothing blocking (R5, R6)
        s.release();
    }
}

// Role: supervising task — launches both workers and joins them (R1).
fn main() {
    let s = Arc::new(Semaphore::new(1)); // exactly one initial permit

    let s_for_w1 = Arc::clone(&s);
    let w1 = thread::spawn(move || worker("w1", s_for_w1));

    let s_for_w2 = Arc::clone(&s);
    let w2 = thread::spawn(move || worker("w2", s_for_w2));

    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // All interleavings terminate (R6); exactly one output line (R7).
    println!("DONE done=1");
}
