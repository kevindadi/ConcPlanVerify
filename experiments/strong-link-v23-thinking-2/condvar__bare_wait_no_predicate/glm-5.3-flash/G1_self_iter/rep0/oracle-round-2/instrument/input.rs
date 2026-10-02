use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // R2: one lock (m) and one condition variable (cv); m guards the boolean flag (ready).
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    // R1: waiter and notifier run concurrently as separate threads.
    let waiter = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || {
            // R6: take the lock only to check/wait; wait releases it while blocked.
            let mut ready = m.lock().unwrap();
            // R4/R9: check the flag first; wait only while false; re-check after each wake.
            while !*ready {
                ready = cv.wait(ready).unwrap();
            }
            // R9: loop exits only when ready is true.
        })
    };

    let notifier = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || {
            // R3: set the flag while holding the lock, signal, then release.
            let mut ready = m.lock().unwrap();
            *ready = true; // R8: flag becomes true in every schedule.
            cv.notify_one();
            // Lock released here when `ready` (the guard) drops.
        })
    };

    // R7: join both roles; every interleaving terminates because the waiter
    // never waits on an already-true flag and re-checks after each wake.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // R10: print exactly `DONE ready=true` and exit.
    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
}
