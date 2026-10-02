// R1: A supervising task (main) launches two worker threads (w1, w2) and waits for both.
// R2: Both workers share one counting permit pool `s` that begins with exactly one permit.
// R3: A worker holds a permit (via a Permit guard) while it performs its work.
// R4: Each worker acquires the permit multiple times; the Permit guard releases it
//     exactly once per acquire on every path (including early exits/panics) via Drop.
// R5: While a worker waits, it holds only the internal mutex inside `wait()`, which is
//     released while blocked, so the current holder can always release the permit.
// R6: The Condvar-based semaphore guarantees every interleaving terminates.
// R7: The program prints exactly `DONE done=1` and exits.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// Counting semaphore with one internal permit pool.
struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

/// Guard representing a held permit. Releases the permit exactly once on drop.
struct Permit<'a> {
    sem: &'a Semaphore,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            permits: Mutex::new(initial),
            available: Condvar::new(),
        }
    }

    /// Blocks until a permit is available, then takes it and returns a guard.
    /// The internal mutex is released while waiting (R5), so a holder can
    /// always release and wake this waiter.
    fn acquire(&self) -> Permit<'_> {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            count = self.available.wait(count).unwrap();
        }
        *count -= 1;
        Permit { sem: self }
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut count = self.sem.permits.lock().unwrap();
        *count += 1;
        self.sem.available.notify_one();
    }
}

/// Worker body: acquires the permit `rounds` times, holding it while working.
/// The guard guarantees release exactly once per acquire on every path (R4).
fn worker(name: &'static str, s: Arc<Semaphore>, rounds: usize) {
    for i in 0..rounds {
        let _permit = s.acquire(); // permit held for the whole work section (R3)
        // Perform work while holding the permit (no output allowed here).
        std::hint::black_box((name, i));
    } // _permit dropped here: permit released (R4)
}

fn main() {
    // R2: one shared permit pool starting with exactly one permit.
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    // R1: supervising task launches w1 and w2.
    let h1 = thread::spawn(move || worker("w1", s1, 5));
    let h2 = thread::spawn(move || worker("w2", s2, 3));

    // R1: wait for both workers to finish.
    h1.join().unwrap();
    h2.join().unwrap();

    // R7: exactly one output line, then exit.
    println!("DONE done=1");
}
