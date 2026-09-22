use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::thread;

/// Counting semaphore `s` built from a Mutex + Condvar.
/// Initialized with a single permit, so at most one activation
/// may hold it at any moment (R3); acquirers block while the
/// permit is unavailable (R4).
struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            // Wait until the permit becomes available.
            permits = self.available.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.available.notify_one();
    }
}

/// One unit of work performed while holding the single permit.
fn do_work(work: &AtomicUsize) {
    for _ in 0..3 {
        work.fetch_add(1, Ordering::SeqCst);
    }
}

// Worker role w1: each activation holds the permit while working (R2).
fn w1(s: &Semaphore, work: &AtomicUsize) {
    s.acquire();
    do_work(work);
    s.release();
}

// Worker role w2.
fn w2(s: &Semaphore, work: &AtomicUsize) {
    s.acquire();
    do_work(work);
    s.release();
}

// Worker role w3.
fn w3(s: &Semaphore, work: &AtomicUsize) {
    s.acquire();
    do_work(work);
    s.release();
}

fn main() {
    // Shared resource: semaphore `s` with exactly one permit.
    let s = Semaphore::new(1);
    let work = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);

    // Scoped threads guarantee every spawned activation is joined
    // before the scope exits (R5, R6).
    thread::scope(|scope| {
        // Each role may have up to two activations running at once (R1).
        for _ in 0..2 {
            scope.spawn(|| w1(&s, &work));
            scope.spawn(|| w2(&s, &work));
            scope.spawn(|| w3(&s, &work));
        }
    });

    // All three roles have completed.
    done.store(1, Ordering::SeqCst);
    println!("DONE done={}", done.load(Ordering::SeqCst));
}
