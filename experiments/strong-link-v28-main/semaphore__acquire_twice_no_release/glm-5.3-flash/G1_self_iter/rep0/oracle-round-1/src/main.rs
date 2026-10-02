mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

/// Counting permit pool (R2): starts with an exact number of permits.
struct Semaphore {
    state: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            state: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    /// Blocks until a permit is available. Returns an RAII guard so the
    /// permit is released exactly once on every path (R3, R4).
    fn acquire(self: &Arc<Self>) -> PermitGuard {
        let mut p = self.state.lock().unwrap();
        while *p == 0 {
            // Spurious-wakeup safe: re-check the predicate (defect 4).
            p = self.available.wait(p).unwrap();
        }
        *p -= 1;
        PermitGuard { s: Arc::clone(self) }
    }

    /// Brief lock, mutate, unlock, then notify — never signals while
    /// holding the lock, so a waiting worker can never block the
    /// releaser (R5, defect 1).
    fn release(&self) {
        {
            let mut p = self.state.lock().unwrap();
            *p += 1;
        }
        self.available.notify_one();
    }

    fn permits(&self) -> usize {
        *self.state.lock().unwrap()
    }
}

/// RAII permit: Drop runs on normal exit, early return, and panic
/// unwinding, guaranteeing acquire/release balance (R4, defect 2).
struct PermitGuard {
    s: Arc<Semaphore>,
}

impl Drop for PermitGuard {
    fn drop(&mut self) {
        self.s.release();
    }
}

const ROUNDS: usize = 3; // each worker acquires more than once (R4)

fn work(id: &str, i: usize) {
    // Critical section: only one worker is ever here (R3).
    // No output permitted except the final DONE line.
    let _ = (id, i);
}

fn w1(s: Arc<Semaphore>) {
    for i in 0..ROUNDS {
        // Named binding (NOT `_`): guard lives to end of iteration and
        // is dropped exactly once per acquire (defect 3).
        let _permit = s.acquire();
        work("w1", i);
    } // released here on every path
}

fn w2(s: Arc<Semaphore>) {
    for i in 0..ROUNDS {
        let _permit = s.acquire();
        work("w2", i);
    }
}

/// Supervising task (R1): launches w1 and w2, waits for both.
fn supervisor(s: Arc<Semaphore>) {
    let h1 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w1#2298", move || w1(s))
    };
    let h2 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w2#2389", move || w2(s))
    };
    // Ignore join errors so a panicked worker cannot prevent the other
    // worker's completion from being observed (R6).
    let _ = h1.join();
    let _ = h2.join();
}

fn main() { cir_trace::init();
    // R2: exactly one permit initially.
    let s = Arc::new(Semaphore::new(1));

    let sup = {
        let s = Arc::clone(&s);
        cir_trace::spawn("supervisor#2750", move || supervisor(s))
    };
    sup.join().expect("supervisor must not panic");

    // All acquires are balanced by releases, so the pool is back to 1.
    let done = s.permits();
    println!("DONE done={}", done); // R7: exactly this line, then exit
 cir_trace::finish();}
