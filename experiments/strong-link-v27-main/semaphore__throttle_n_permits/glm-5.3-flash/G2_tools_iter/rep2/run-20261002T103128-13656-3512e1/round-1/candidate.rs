use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Shared resource: s — a counting semaphore.
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

    // Acquire one permit; block while none are available.
    // A waiting worker holds the mutex only while checking/updating the
    // count; `wait` releases the mutex while blocked, so any permit holder
    // can still lock the mutex and release its permit (R5).
    fn acquire(&self) {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }

    // Release one permit and wake a waiting worker, if any.
    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        drop(count);
        self.cv.notify_one();
    }
}

// Worker role: acquire a permit, do the work, release the permit (R3).
fn worker(name: &'static str, s: Arc<Semaphore>) {
    s.acquire();
    // Perform the worker's work while holding a permit.
    // At most two workers can be here at the same time (R4).
    let _ = name;
    s.release();
    // Worker finishes after releasing.
}

fn main() {
    // Supervising task: launches w1, w2, w3 and waits for all of them (R1).
    // The pool starts with exactly two permits (R2).
    let s = Arc::new(Semaphore::new(2));

    let w1 = {
        let s = Arc::clone(&s);
        thread::spawn(move || worker("w1", s))
    };
    let w2 = {
        let s = Arc::clone(&s);
        thread::spawn(move || worker("w2", s))
    };
    let w3 = {
        let s = Arc::clone(&s);
        thread::spawn(move || worker("w3", s))
    };

    // Wait for all workers to finish. Because a waiting worker never
    // prevents a holder from releasing (R5), every interleaving terminates (R6).
    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
}
