mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

/// Counting semaphore with a single permit initially (R2).
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

    /// Blocks until a permit is available. While waiting, the internal
    /// mutex is released, so the current holder can still `release` (R5).
    fn acquire(&self) {
        let mut n = self.permits.lock().unwrap();
        while *n == 0 {
            // `wait` atomically releases the mutex and sleeps;
            // re-check on every wakeup, including spurious ones (R6).
            n = self.available.wait(n).unwrap();
        }
        *n -= 1;
    }

    /// Returns one permit. The mutex is held only briefly; it is
    /// dropped before notifying so the woken waiter never queues
    /// behind the notifier. With one permit and two workers, at most
    /// one thread can be blocked in `wait` at any time, so
    /// `notify_one` cannot lose a wakeup (R6): if nobody is waiting,
    /// the incremented count is still observed by the other worker's
    /// next `acquire`.
    fn release(&self) {
        let mut n = self.permits.lock().unwrap();
        *n += 1;
        drop(n);
        self.available.notify_one();
    }
}

/// RAII guard: releases the permit on every path, including
/// early return or panic (R4).
struct Permit<'a> {
    s: &'a Semaphore,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.s.release();
    }
}

/// Each work unit is a strict acquire -> work -> release cycle.
/// The guard drops at the end of each iteration, so the permit is
/// never held across the next `acquire` call and a worker can never
/// block on itself (R3, R4, R6).
fn worker(name: &'static str, s: Arc<Semaphore>, units: usize) {
    for i in 0..units {
        s.acquire();
        let _permit = Permit { s: &*s };
        // The work itself: pure computation, no output, so the
        // program's only printed line is the final one (R7).
        let _ = (name, i);
        // `_permit` drops here (or on unwind), releasing exactly once.
    }
}

fn main() { cir_trace::init();
    // Shared pool begins with exactly one permit (R2).
    let s = Arc::new(Semaphore::new(1));

    // Supervisor launches both workers (R1).
    let s1 = Arc::clone(&s);
    let w1 = cir_trace::spawn("worker#2447", move || worker("w1", s1, 2));

    let s2 = Arc::clone(&s);
    let w2 = cir_trace::spawn("worker#2534", move || worker("w2", s2, 2));

    // Supervisor waits for both workers to finish (R1, R6).
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // All acquires are matched by releases by the time both threads
    // have been joined, so the count is back to the initial value.
    let done = *s.permits.lock().unwrap();
    println!("DONE done={}", done); // prints exactly `DONE done=1` (R7)
 cir_trace::finish();}
