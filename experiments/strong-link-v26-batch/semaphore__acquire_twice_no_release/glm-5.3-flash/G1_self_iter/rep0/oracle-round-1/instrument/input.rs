use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// Counting semaphore (shared resource `s`).
/// Invariant: `permits` never exceeds the count it was created with.
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

    /// Blocks until a permit is available.
    ///
    /// Defect repaired (R5): the waiting thread does NOT hold the mutex while
    /// blocked. `Condvar::wait` atomically releases the mutex, so a worker
    /// currently holding a permit can always lock the semaphore and release
    /// it, waking the waiter. (A broken version that waits while pinning the
    /// mutex — or that waits on a second lock guarding the same state —
    /// deadlocks the holder out of `release`.)
    fn acquire(self: &Arc<Self>) -> Permit {
        let mut n = self.permits.lock().expect("semaphore mutex poisoned");
        while *n == 0 {
            n = self.available.wait(n).expect("semaphore mutex poisoned");
        }
        *n -= 1;
        Permit { sem: Arc::clone(self) }
    }

    /// Defect repaired: `release` only takes the mutex briefly and never
    /// blocks on worker progress, so it cannot be starved by a waiter.
    fn release(&self) {
        let mut n = self.permits.lock().expect("semaphore mutex poisoned");
        *n += 1;
        drop(n);
        self.available.notify_one();
    }
}

/// RAII permit guard.
///
/// Defect repaired (R4): release is tied to drop, so the permit is returned
/// exactly once on *every* path — normal end of scope, early `return`, `?`,
/// or panic unwinding. Manual acquire/release pairs (the classic defect)
/// leak a permit on any early exit and permanently starve the other worker.
/// The guard owns an `Arc` clone, so the semaphore always outlives it.
struct Permit {
    sem: Arc<Semaphore>,
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.sem.release();
    }
}

/// Worker body (roles `w1` and `w2`).
fn worker(sem: Arc<Semaphore>) {
    // R4: each worker acquires the permit more than once (two cycles),
    // and each acquisition is balanced by exactly one release.
    for _ in 0..2 {
        let _permit = sem.acquire(); // R3: permit held for the whole work section
        for _ in 0..64 {
            std::hint::spin_loop();
        }
        std::thread::yield_now();
    } // <- permit released here on every iteration, every path
}

fn main() {
    // R2: one shared pool, beginning with exactly one permit.
    let s = Arc::new(Semaphore::new(1));

    // R1: the supervisor (main) launches w1 and w2.
    let w1 = {
        let s = Arc::clone(&s);
        thread::Builder::new()
            .name("w1".to_string())
            .spawn(move || worker(s))
            .expect("failed to spawn w1")
    };
    let w2 = {
        let s = Arc::clone(&s);
        thread::Builder::new()
            .name("w2".to_string())
            .spawn(move || worker(s))
            .expect("failed to spawn w2")
    };

    // R1/R6: supervisor waits for both workers. `join` returns even if a
    // worker panicked, because its `Permit` was still released during
    // unwinding — so the other worker can never be stuck waiting forever.
    let _ = w1.join();
    let _ = w2.join();

    // R7: `done` is the supervisor's completion flag. Both workers have
    // terminated at this point on every schedule, so it is always 1.
    let done: u32 = 1;
    println!("DONE done={done}");
}
