use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // R2: one lock (m) guarding a boolean flag (ready), plus one condition variable (cv).
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    // R1: spawn the waiter and notifier roles so they run at the same time.
    let waiter = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            let (m, cv) = &*shared;
            // R4: check the flag first while holding the lock.
            let mut ready = m.lock().unwrap();
            // R4/R6: wait only while the flag is false; re-check after each wake.
            while !*ready {
                // R6: cv.wait releases the lock while blocked and reacquires it on wake.
                ready = cv.wait(ready).unwrap();
            }
            // R9: we only get here once the flag is true.
        })
    };

    let notifier = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            let (m, cv) = &*shared;
            // R3: set the flag while holding the lock...
            let mut ready = m.lock().unwrap();
            *ready = true;
            // ...signal the condition variable...
            cv.notify_all();
            // ...then release the lock (guard dropped at end of scope).
        })
    };

    // R7: join both roles so every interleaving terminates with both finished.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // R8: the flag is true in every schedule; verify and print exactly once.
    let ready = *shared.0.lock().unwrap();
    // R10: print exactly `DONE ready=true` and exit.
    println!("DONE ready={}", ready);
}
