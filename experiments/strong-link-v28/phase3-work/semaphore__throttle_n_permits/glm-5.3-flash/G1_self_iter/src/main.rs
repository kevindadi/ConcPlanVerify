mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

/// Counting semaphore: `permits` permits, guarded by a mutex that is
/// held only while reading/updating the count — never while a worker
/// performs its work (R5).
struct Semaphore {
    permits: Mutex<u32>,
    cond: Condvar,
}

impl Semaphore {
    fn new(n: u32) -> Self {
        Semaphore {
            permits: Mutex::new(n),
            cond: Condvar::new(),
        }
    }

    /// Acquire one permit, blocking until one is available.
    fn acquire(&self) {
        // Lock is taken only for the check/decrement, then dropped.
        let mut count = self.permits.lock().unwrap();
        // `while`, not `if`: re-check after every wakeup so spurious
        // wakeups and racing releasers cannot let us run with 0 permits (R6).
        while *count == 0 {
            // wait() atomically releases the mutex and sleeps, so a
            // permit holder can still acquire the lock and release (R5).
            count = self.cond.wait(count).unwrap();
        }
        *count -= 1;
        // Mutex guard dropped here — before any work is done.
    }

    /// Release one permit and wake a waiting worker, if any.
    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        // Notify while holding the lock: no lost wakeup between the
        // count increment and the waiter's re-check (R4, R5).
        self.cond.notify_one();
    }
}

fn worker(name: &'static str, s: Arc<Semaphore>) {
    s.acquire();
    // Critical section: at most two workers are here simultaneously (R4).
    // The semaphore's internal mutex is NOT held here (R5).
    println!("{} working", name);
    s.release();
    // Worker finishes after releasing (R3).
}

fn main() { cir_trace::init();
    // Shared permit pool with exactly two permits (R2).
    let s = Arc::new(Semaphore::new(2));

    // Supervising task: launch w1, w2, w3 and wait for all of them (R1).
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let s3 = Arc::clone(&s);

    let h1 = cir_trace::spawn("worker#2051", move || worker("w1", s1));
    let h2 = cir_trace::spawn("worker#2105", move || worker("w2", s2));
    let h3 = cir_trace::spawn("worker#2159", move || worker("w3", s3));

    // Joining all three guarantees every schedule terminates (R6):
    // a waiting worker is always woken by a holder's release+notify,
    // and the supervisor never exits before the workers do.
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    // All workers finished; the run is done.
    let done = 1;
    println!("DONE done={}", done); // exactly `DONE done=1` (R7)
 cir_trace::finish();}
