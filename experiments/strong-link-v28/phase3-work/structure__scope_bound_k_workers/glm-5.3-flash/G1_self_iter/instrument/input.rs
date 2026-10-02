use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// Counting semaphore `s`, initialized with a single permit (R1, R3).
struct Semaphore {
    count: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            count: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    /// R4: block until the permit is available; R3: decrement under the lock
    /// so at most one activation ever holds it.
    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        // `while`, not `if`: guards against spurious wakeups and against a
        // woken thread finding the permit already taken (defect 1).
        while *count == 0 {
            count = self.available.wait(count).unwrap();
        }
        *count -= 1;
    }

    /// R2: release the permit afterwards; notify under the lock so no
    /// wakeup is lost (defect 2).
    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        drop(count);
        self.available.notify_one();
    }
}

/// Each worker role spawns up to two concurrent activations (R1),
/// each of which holds `s` while working and releases it after (R2).
fn worker(s: Arc<Semaphore>) {
    let mut activations = Vec::new();
    for _ in 0..2 {
        let s = Arc::clone(&s);
        activations.push(thread::spawn(move || {
            s.acquire();            // R2, R4
            // ... work while holding the single permit ...
            s.release();            // R2
        }));
    }
    // Join both activations so the worker only completes when its
    // activations are done (defect 3; R5, R6).
    for a in activations {
        a.join().unwrap();
    }
}

fn main() {
    let s = Arc::new(Semaphore::new(1)); // single shared permit

    // Main task starts the three worker roles w1, w2, w3 (R1).
    let mut workers = Vec::new();
    for _name in ["w1", "w2", "w3"] {
        let s = Arc::clone(&s);
        workers.push(thread::spawn(move || worker(s)));
    }

    // Join all workers: every role completes (R6) and every
    // schedule terminates (R5) before anything is printed.
    for w in workers {
        w.join().unwrap();
    }

    // Exactly one line of output, then exit (R7). No worker prints
    // anything, so no interleaving can add extra lines (defect 4).
    println!("DONE done=1");
}
