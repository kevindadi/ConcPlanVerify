use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // R2: one lock (m) and one condition variable (cv), guarding the boolean flag (ready).
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    // waiter role
    let waiter = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            // Access the lock (m) and condition variable (cv) through the shared Arc.
            let (m, cv) = &*shared;
            // R4: check the flag first while holding the lock.
            let mut guard = m.lock().unwrap();
            // R4/R6: wait only while the flag is false; re-check after each wake.
            // R5: if the notifier already set the flag, we never enter wait.
            // R6: cv.wait releases the lock while blocked, reacquires on wake.
            while !*guard {
                guard = cv.wait(guard).unwrap();
            }
            // R9: we only get here once ready is true.
            // R10: print exactly the required line.
            println!("DONE ready=true");
        })
    };

    // notifier role
    let notifier = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            // Access the lock (m) and condition variable (cv) through the shared Arc.
            let (m, cv) = &*shared;
            // R3: set the flag while holding the lock...
            let mut guard = m.lock().unwrap();
            *guard = true;
            // ...signal the condition variable while still holding the lock...
            cv.notify_all();
            // ...then release the lock.
            drop(guard);
        })
    };

    // R1: both roles run concurrently; R7: joining both guarantees termination
    // on every interleaving (the predicate loop makes early signals safe).
    waiter.join().unwrap();
    notifier.join().unwrap();
}
