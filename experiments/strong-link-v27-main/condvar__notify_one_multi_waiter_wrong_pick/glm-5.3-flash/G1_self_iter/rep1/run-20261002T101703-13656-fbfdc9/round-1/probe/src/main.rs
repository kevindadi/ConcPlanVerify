use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// Counting semaphore built from a mutex + condition variable.
struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            permits: Mutex::new(initial),
            cv: Condvar::new(),
        }
    }

    fn release(&self) {
        let mut p = self.permits.lock().unwrap();
        *p += 1;
        drop(p);
        self.cv.notify_one();
    }

    fn acquire(&self) {
        let mut p = self.permits.lock().unwrap();
        while *p == 0 {
            p = self.cv.wait(p).unwrap();
        }
        *p -= 1;
    }
}

/// Shared resources: m (lock), cv (condition variable), g12, gN (semaphores).
struct Shared {
    /// m guards the "proceed" flag; cv is waited on under m.
    m: Mutex<bool>,
    cv: Condvar,
    /// g12: one permit released per waiter once it is ready to wait.
    g12: Semaphore,
    /// gN: one permit released per waiter once it has finished.
    gN: Semaphore,
}

/// Role: w1 / w2 — block on cv under the lock until told to proceed.
fn waiter(shared: Arc<Shared>) {
    // R4: hold the lock while (preparing to) wait on the condition variable.
    let mut proceed = shared.m.lock().unwrap();

    // R6: announce readiness *while holding m*. The notifier cannot then
    // acquire m until this thread is blocked inside cv.wait, so the
    // notification below can never be missed.
    shared.g12.release();

    // R2: block until the notifier tells us to proceed.
    while !*proceed {
        proceed = shared.cv.wait(proceed).unwrap();
    }

    // Lock released on drop; record completion for the main task.
    drop(proceed);
    shared.gN.release();
}

/// Role: notifier — wake every waiter that is still blocked.
fn notifier(shared: Arc<Shared>) {
    // R6: wait until both waiters are ready to wait (two permits).
    shared.g12.acquire();
    shared.g12.acquire();

    // R5: take the lock before waking the waiters...
    let mut proceed = shared.m.lock().unwrap();
    *proceed = true;

    // R7: a single wakeup is not enough — wake *every* blocked waiter.
    shared.cv.notify_all();

    // R5: ...and release the lock afterwards.
    drop(proceed);
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(false),
        cv: Condvar::new(),
        g12: Semaphore::new(0),
        gN: Semaphore::new(0),
    });

    // R1: w1, w2 and notifier run at the same time.
    let w1 = {
        let s = Arc::clone(&shared);
        thread::spawn(move || waiter(s))
    };
    let w2 = {
        let s = Arc::clone(&shared);
        thread::spawn(move || waiter(s))
    };
    let n = {
        let s = Arc::clone(&shared);
        thread::spawn(move || notifier(s))
    };

    // R8/R9: both waiters have completed (two gN permits) before reporting.
    shared.gN.acquire();
    shared.gN.acquire();

    // Ensure all roles have fully terminated before exiting.
    w1.join().unwrap();
    w2.join().unwrap();
    n.join().unwrap();

    // R10: exactly this line, then exit.
    println!("DONE waiters=0");
}
