use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// Minimal counting semaphore (permits + condvar), used for g12 and gN.
struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(n: usize) -> Self {
        Semaphore { permits: Mutex::new(n), cv: Condvar::new() }
    }

    fn acquire(&self) {
        let mut p = self.permits.lock().unwrap();
        while *p == 0 {
            p = self.cv.wait(p).unwrap();
        }
        *p -= 1;
    }

    fn release(&self) {
        *self.permits.lock().unwrap() += 1;
        self.cv.notify_one();
    }
}

fn main() {
    // m doubles as the lock guarding cv's predicate (the `go` flag).
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());
    let g12 = Arc::new(Semaphore::new(0)); // readiness permits from w1/w2
    let gN = Arc::new(Semaphore::new(0)); // notifier-finished permit

    let mut handles = Vec::new();

    // R1: two waiter roles, w1 and w2.
    for name in ["w1", "w2"] {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        handles.push(thread::spawn(move || {
            // R4: take the lock before waiting on the condition variable.
            let mut go = m.lock().unwrap();
            // R6: signal "ready to wait" while still holding m, so the
            // notifier cannot proceed until we are (about to be) blocked.
            g12.release();
            // R2: block until told to proceed; loop guards against
            // spurious wakeups.
            while !*go {
                go = cv.wait(go).unwrap();
            }
            drop(go);
            let _ = name; // role label; no output required from waiters
        }));
    }

    // R1: one notifier role, running concurrently with the waiters.
    {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        handles.push(thread::spawn(move || {
            // R6: wait until BOTH waiters are ready to wait.
            g12.acquire();
            g12.acquire();
            // R5: take the lock before waking the waiters.
            let mut go = m.lock().unwrap();
            *go = true;          // predicate change under the same lock
            cv.notify_all();     // R3 + R7: wake EVERY blocked waiter
            drop(go);            // R5: release the lock afterwards
            gN.release();        // tell main the wake-up is complete
        }));
    }

    // Wait until the notifier has woken everyone, then let all roles finish.
    gN.acquire();
    for h in handles {
        h.join().unwrap();
    }

    // R10: all waiters have exited, so none remain waiting.
    println!("DONE waiters=0");
}
