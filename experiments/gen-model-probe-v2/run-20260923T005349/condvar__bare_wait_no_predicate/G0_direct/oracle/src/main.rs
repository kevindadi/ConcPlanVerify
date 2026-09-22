mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn waiter(pair: Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*pair;
    // Acquire the lock guarding `ready`.
    let mut ready = m.lock().unwrap();
    // Check the flag first; wait only while it is false, and re-check
    // after every wake. `Condvar::wait` releases the lock while blocked
    // and re-acquires it before returning.
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
    // At this point `ready` is guaranteed to be true.
    println!("DONE ready={}", *ready);
}

fn notifier(pair: Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*pair;
    {
        // Set the shared flag to true while holding the lock.
        let mut ready = m.lock().unwrap();
        *ready = true;
        // Signal the condition variable while still holding the lock.
        cv.notify_one();
        // The lock is released here when the guard goes out of scope.
    }
}

fn main() { cir_trace::init();
    // Shared state: one lock `m`, one condition variable `cv`,
    // and the boolean flag `ready` guarded by `m`.
    let shared = Arc::new((Mutex::new_named("shared_mutex0", false), Condvar::new_named("shared_condvar0")));

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    // Start the waiter and notifier roles concurrently.
    let waiter_handle = cir_trace::spawn("waiter_handle", move || waiter(waiter_shared));
    let notifier_handle = cir_trace::spawn("notifier_handle", move || notifier(notifier_shared));

    // Ensure both roles finish before main terminates.
    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();
 cir_trace::finish();}
