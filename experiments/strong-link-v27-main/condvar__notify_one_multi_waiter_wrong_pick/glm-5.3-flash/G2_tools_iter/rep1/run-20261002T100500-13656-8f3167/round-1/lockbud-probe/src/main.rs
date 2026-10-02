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

    /// Block until a permit is available, then take it.
    fn wait(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.cv.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    /// Release one permit.
    fn signal(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.cv.notify_one();
    }
}

fn main() {
    // Shared resources:
    // m  -- the lock guarding the "proceed" flag
    // cv -- the condition variable the waiters block on
    // g12 -- permit counter: both waiters signal it once they hold m and are
    //        about to wait; the notifier waits for both permits before waking.
    // gN -- permit counter: each waiter signals it when finished; main waits
    //       for both before printing the final line.
    let m = Arc::new(Mutex::new(false)); // false = not yet told to proceed
    let cv = Arc::new(Condvar::new());
    let g12 = Arc::new(Semaphore::new(0));
    let gN = Arc::new(Semaphore::new(0));

    let mut handles = Vec::new();

    // Spawn the two waiters (w1, w2).
    for _ in 0..2 {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);

        handles.push(thread::spawn(move || {
            // R4: the waiter must hold the lock while it waits on cv.
            let mut guard = m.lock().unwrap();

            // R6: announce (while holding m) that we are ready to wait.
            g12.signal();

            // Block until the notifier tells us to proceed.
            while !*guard {
                guard = cv.wait(guard).unwrap();
            }

            // Release the lock before leaving.
            drop(guard);

            // Tell main that this waiter has finished.
            gN.signal();
        }));
    }

    // Spawn the notifier.
    {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);

        handles.push(thread::spawn(move || {
            // R6: only proceed once BOTH waiters are ready to wait.
            g12.wait();
            g12.wait();

            // R5: take the lock before waking the waiters...
            let _guard = m.lock().unwrap();

            // R3 + R7: wake every blocked waiter, not just one.
            cv.notify_all();

            // ...and release the lock afterwards (guard drops here).
        }));
    }

    // Main waits until both waiters have finished (R8, R9).
    gN.wait();
    gN.wait();

    for handle in handles {
        handle.join().unwrap();
    }

    // R10: all waiters have completed, so none remain.
    let remaining = 0;
    println!("DONE waiters={}", remaining);
}
