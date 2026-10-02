mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use std::time::Duration;

// Counting permit pool `s` with exactly two permits.
// Implemented with a Mutex + Condvar so that a waiting worker blocks on the
// condition variable (not on the mutex), which means a permit holder can
// always acquire the mutex and release its permit (R5).
struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(n: usize) -> Semaphore {
        Semaphore {
            permits: Mutex::new(n),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            // Wait releases the mutex while blocked, so holders can release.
            permits = self.cv.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.cv.notify_one();
    }
}

fn worker(name: &'static str, s: Arc<Semaphore>) {
    // R3: acquire one permit, do the work, release the permit.
    s.acquire();
    // Simulate the worker's work while holding the permit.
    thread::sleep(Duration::from_millis(50));
    s.release();
    let _ = name; // role name kept for clarity; no extra output allowed (R7)
}

fn main() { cir_trace::init();
    // R2: one shared permit pool starting with exactly two permits.
    let s = Arc::new(Semaphore::new(2));

    // R1: supervising task launches w1, w2, w3 and waits for all of them.
    let s1 = Arc::clone(&s);
    let h1 = cir_trace::spawn("worker#1567", move || worker("w1", s1));
    let s2 = Arc::clone(&s);
    let h2 = cir_trace::spawn("worker#1650", move || worker("w2", s2));
    let s3 = Arc::clone(&s);
    let h3 = cir_trace::spawn("worker#1733", move || worker("w3", s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    // R7: print exactly this line, then exit.
    println!("DONE done=1");
 cir_trace::finish();}
