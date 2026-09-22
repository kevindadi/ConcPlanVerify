use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// A simple counting semaphore built from a mutex and a condition variable.
struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Mutex::new(permits),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut guard = self.permits.lock().unwrap();
        while *guard == 0 {
            guard = self.cv.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    fn release(&self) {
        let mut guard = self.permits.lock().unwrap();
        *guard += 1;
        self.cv.notify_one();
    }
}

struct Shared {
    m: Mutex<bool>,      // m: the lock guarding the "proceed" flag
    cv: Condvar,         // cv: the shared condition variable
    g12: Semaphore,      // g12: waiters -> notifier, "ready to wait" permits
    gN: Semaphore,       // gN: waiters -> main, "finished" permits
    waiters: AtomicUsize,
}

fn waiter(shared: Arc<Shared>) {
    // R4: the waiter holds the lock while it waits on the condition variable.
    let mut go = shared.m.lock().unwrap();

    // R6: announce (while still holding m) that this waiter is ready to wait.
    // Because m is held until `wait` atomically releases it, the notifier
    // cannot slip in and set the flag before this waiter is actually blocked.
    shared.g12.release();

    // R2: block until told to proceed.
    while !*go {
        go = shared.cv.wait(go).unwrap();
    }
    drop(go);

    shared.waiters.fetch_sub(1, Ordering::SeqCst);
    shared.gN.release();
}

fn notifier(shared: Arc<Shared>) {
    // R6: only wake the waiters after both are ready to wait.
    shared.g12.acquire();
    shared.g12.acquire();

    // R5: take the lock before waking, release it afterwards.
    let mut go = shared.m.lock().unwrap();
    *go = true;
    // R3 / R7: wake every waiter that is still blocked, not just one.
    shared.cv.notify_all();
    drop(go);
}

fn main() {
    // R1: start two waiter roles and one notifier role concurrently.
    let shared = Arc::new(Shared {
        m: Mutex::new(false),
        cv: Condvar::new(),
        g12: Semaphore::new(0),
        gN: Semaphore::new(0),
        waiters: AtomicUsize::new(2),
    });

    let w1 = {
        let s = Arc::clone(&shared);
        thread::Builder::new()
            .name("w1".to_string())
            .spawn(move || waiter(s))
            .unwrap()
    };
    let w2 = {
        let s = Arc::clone(&shared);
        thread::Builder::new()
            .name("w2".to_string())
            .spawn(move || waiter(s))
            .unwrap()
    };
    let notifier_handle = {
        let s = Arc::clone(&shared);
        thread::Builder::new()
            .name("notifier".to_string())
            .spawn(move || notifier(s))
            .unwrap()
    };

    // Wait until both waiters have completed (R8/R9).
    shared.gN.acquire();
    shared.gN.acquire();

    w1.join().unwrap();
    w2.join().unwrap();
    notifier_handle.join().unwrap();

    // R10: exactly this line, then exit.
    println!("DONE waiters={}", shared.waiters.load(Ordering::SeqCst));
}
