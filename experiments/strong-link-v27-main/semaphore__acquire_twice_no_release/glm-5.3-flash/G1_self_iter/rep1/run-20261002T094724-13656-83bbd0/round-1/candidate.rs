use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// Counting permit pool (semaphore), starting with a fixed number of permits.
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

    /// Blocks until a permit is available. The mutex is released while
    /// waiting (Condvar::wait), so the current holder can always release.
    fn acquire(&self) -> PermitGuard<'_> {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.available.wait(permits).unwrap();
        }
        *permits -= 1;
        PermitGuard { sem: self }
    }
}

struct PermitGuard<'a> {
    sem: &'a Semaphore,
}

// RAII: the permit is released exactly once per acquisition on *every*
// path, including early returns and panics (R4).
impl Drop for PermitGuard<'_> {
    fn drop(&mut self) {
        let mut permits = self.sem.permits.lock().unwrap();
        *permits += 1;
        drop(permits);
        // Wake a waiter so every schedule terminates (R5, R6).
        self.sem.available.notify_one();
    }
}

fn worker(s: &Arc<Semaphore>) {
    // Each worker acquires the permit more than once (R4), and the two
    // workers never hold it at the same time (R3). The guard is dropped
    // at the end of each block before the next acquisition, so a worker
    // never deadlocks against itself.
    for _ in 0..2 {
        let _permit = s.acquire();
        // Hold permit while performing work (R3).
        std::hint::black_box(42);
    } // guard dropped here: release #1 and #2, exactly once each
}

fn main() {
    // Shared counting permit pool with exactly one permit (R2).
    let s = Arc::new(Semaphore::new(1));

    // Supervising task launches both workers and waits for both (R1).
    let h1 = {
        let s = Arc::clone(&s);
        thread::spawn(move || worker(&s)) // w1
    };
    let h2 = {
        let s = Arc::clone(&s);
        thread::spawn(move || worker(&s)) // w2
    };

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // Exactly one final line (R7).
    println!("DONE done=1");
}
