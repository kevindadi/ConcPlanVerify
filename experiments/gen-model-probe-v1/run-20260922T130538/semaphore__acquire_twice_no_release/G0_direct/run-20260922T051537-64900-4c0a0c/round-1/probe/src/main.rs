use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// A counting semaphore implemented with a mutex and a condition variable.
/// The pool starts with a fixed number of permits; `acquire` blocks until
/// one is available, and `release` returns one and wakes a waiter.
struct PermitPool {
    permits: Mutex<usize>,
    available: Condvar,
}

impl PermitPool {
    fn new(permits: usize) -> Self {
        PermitPool {
            permits: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut guard = self.permits.lock().unwrap();
        while *guard == 0 {
            // Waiting releases the mutex, so the current permit holder
            // remains able to enter `release` and wake us up.
            guard = self.available.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    fn release(&self) {
        let mut guard = self.permits.lock().unwrap();
        *guard += 1;
        self.available.notify_one();
    }
}

/// Each worker performs its work in bounded critical sections, acquiring
/// and releasing the shared permit once per section. Every acquire is
/// matched by exactly one release on every path.
fn worker(rounds: usize, pool: Arc<PermitPool>, completed: Arc<AtomicUsize>) {
    for _ in 0..rounds {
        pool.acquire();
        // Critical section: at most one worker can be here at any time,
        // because the pool holds exactly one permit.
        completed.fetch_add(1, Ordering::SeqCst);
        pool.release();
    }
}

fn main() {
    // Supervising task: launches the two workers and waits for both.
    let supervisor = thread::spawn(|| {
        // One shared counting permit pool with exactly one permit.
        let pool = Arc::new(PermitPool::new(1));
        let completed = Arc::new(AtomicUsize::new(0));

        let rounds_per_worker = 2;
        let mut workers = Vec::new();
        for _ in 0..2 {
            let pool = Arc::clone(&pool);
            let completed = Arc::clone(&completed);
            workers.push(thread::spawn(move || {
                worker(rounds_per_worker, pool, completed);
            }));
        }

        // Wait for both workers to finish.
        for handle in workers {
            handle.join().unwrap();
        }

        // Both workers finished all of their critical sections exactly once
        // per acquisition, so the total is fully determined.
        let total = completed.load(Ordering::SeqCst);
        let done = if total == 2 * rounds_per_worker { 1 } else { 0 };
        println!("DONE done={}", done);
    });

    supervisor.join().unwrap();
}
