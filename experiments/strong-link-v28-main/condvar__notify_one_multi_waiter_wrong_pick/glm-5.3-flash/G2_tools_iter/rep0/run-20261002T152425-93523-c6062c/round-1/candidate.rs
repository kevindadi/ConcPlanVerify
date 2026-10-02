// R1: main starts two waiters (w1, w2) and one notifier, all running concurrently.
// R2: each waiter blocks on the shared condition variable cv until told to proceed.
// R3: the notifier wakes every waiter still blocked (notify_all, not notify_one).
// R4: a waiter holds the lock m while it waits on cv.
// R5: the notifier takes the lock before waking and releases it afterwards.
// R6: g12 ensures the notifier only wakes the waiters after BOTH are ready to wait.
// R7: waking a single waiter is not enough; notify_all wakes every blocked waiter.
// R8/R9: every schedule terminates with every waiter and the notifier finished.
// R10: the program prints exactly `DONE waiters=0` and exits.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// A simple counting semaphore built from a mutex and a condition variable.
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

    /// Acquire one permit, blocking until one is available.
    fn wait(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.cv.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    /// Release one permit, waking a blocked acquirer if any.
    fn post(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        drop(permits);
        self.cv.notify_one();
    }
}

/// Shared state: the lock m, the condition variable cv, the "go" flag that
/// tells waiters to proceed, and a count of waiters that have finished.
struct Shared {
    m: Mutex<bool>, // guarded bool: has the notifier told waiters to go?
    cv: Condvar,
    finished: Mutex<usize>, // number of waiters that have completed
}

fn waiter(id: usize, shared: &Arc<Shared>, g12: &Arc<Semaphore>, gN: &Arc<Semaphore>) {
    // R6: announce that this waiter is about to block on the condition variable.
    g12.post();

    // R4: take the lock and hold it while waiting on the condition variable.
    let mut go = shared.m.lock().unwrap();
    while !*go {
        // R2: block on cv until the notifier tells us to proceed.
        // cv.wait atomically releases the lock and re-acquires it before returning,
        // so the lock is held whenever we are not actually suspended.
        go = shared.cv.wait(go).unwrap();
    }
    drop(go); // release the lock after waking

    // R9: this waiter completes; record it and hand a permit to the notifier.
    {
        let mut finished = shared.finished.lock().unwrap();
        *finished += 1;
    }
    gN.post();

    let _ = id; // w1 / w2 role identifier
}

fn notifier(shared: &Arc<Shared>, g12: &Arc<Semaphore>, gN: &Arc<Semaphore>) {
    // R6: wait until BOTH waiters are ready to wait on the condition variable.
    g12.wait();
    g12.wait();

    // R5: take the lock before waking the waiters...
    let mut go = shared.m.lock().unwrap();
    *go = true;
    // R3 + R7: wake EVERY waiter that is still blocked, not just one.
    shared.cv.notify_all();
    // R5: ...and release the lock afterwards.
    drop(go);

    // R8: wait for both waiters to actually finish before reporting.
    gN.wait();
    gN.wait();

    // R10: all waiters are done, so the remaining-waiter count is 0.
    let finished = *shared.finished.lock().unwrap();
    let remaining = 2 - finished;
    println!("DONE waiters={}", remaining);
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(false),
        cv: Condvar::new(),
        finished: Mutex::new(0),
    });
    // g12 starts at 0: the notifier must collect two posts (one per waiter)
    // before it is allowed to wake anyone (R6).
    let g12 = Arc::new(Semaphore::new(0));
    // gN starts at 0: the notifier collects one post per finished waiter (R8).
    let gN = Arc::new(Semaphore::new(0));

    // R1: spawn w1, w2, and notifier so they run at the same time.
    let s1 = Arc::clone(&shared);
    let g12a = Arc::clone(&g12);
    let gNa = Arc::clone(&gN);
    let w1 = thread::spawn(move || waiter(1, &s1, &g12a, &gNa));

    let s2 = Arc::clone(&shared);
    let g12b = Arc::clone(&g12);
    let gNb = Arc::clone(&gN);
    let w2 = thread::spawn(move || waiter(2, &s2, &g12b, &gNb));

    let s3 = Arc::clone(&shared);
    let g12c = Arc::clone(&g12);
    let gNc = Arc::clone(&gN);
    let n = thread::spawn(move || notifier(&s3, &g12c, &gNc));

    // R8: joining all three guarantees every schedule terminates with
    // every waiter and the notifier finished.
    w1.join().unwrap();
    w2.join().unwrap();
    n.join().unwrap();
}
