use std::sync::{Arc, Condvar, Mutex};

/// Counting permit pool (semaphore) shared by w1 and w2.
struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(n: usize) -> Self {
        Semaphore {
            permits: Mutex::new(n),
            available: Condvar::new(),
        }
    }

    /// Block until a permit is available, then take it.
    /// The waiter sleeps (does not spin), so the current holder
    /// remains able to run and release (R5).
    fn acquire(&self) {
        let mut count = self.permits.lock().expect("permits mutex poisoned");
        while *count == 0 {
            // wait() atomically releases the mutex and sleeps;
            // re-check on every wakeup (spurious or real).
            count = self.available.wait(count).expect("permits mutex poisoned");
        }
        *count -= 1;
    }

    /// Return one permit and wake a waiting worker, if any.
    fn release(&self) {
        let mut count = self.permits.lock().expect("permits mutex poisoned");
        *count += 1;
        // At most one worker can be waiting, but notify_all is
        // harmless and robust to future changes.
        self.available.notify_all();
    }

    fn count(&self) -> usize {
        *self.permits.lock().expect("permits mutex poisoned")
    }
}

/// Worker body: acquire one permit, do the work, release it
/// before finishing (R3, R4). Release happens even if the
/// work section panics, so no permit can leak (R6).
fn worker(name: &'static str, s: &Semaphore) {
    s.acquire();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // --- work section: only one worker is ever here ---
        println!("{name} working");
    }));
    s.release();
    if let Err(_panic) = result {
        // Propagate after releasing, so the supervisor still observes it.
        std::panic::resume_unwind(_panic);
    }
}

fn main() {
    // Supervising task: launches w1 and w2 and waits for both (R1).
    // One shared pool starting with exactly one permit (R2).
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let h1 = std::thread::spawn(move || worker("w1", &s1));
    let h2 = std::thread::spawn(move || worker("w2", &s2));

    // Join both workers: every schedule terminates (R6), because
    // the blocked waiter never prevents the holder from releasing.
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // Read the count only after both releases are complete,
    // so the printed value is deterministic (R7).
    println!("DONE permits={}", s.count());
}
