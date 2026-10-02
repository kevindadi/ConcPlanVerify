use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    ready: Mutex<bool>,
    cv: Condvar,
}

fn waiter(shared: &Shared) {
    // R4/R6: acquire the lock, check the predicate first.
    let mut ready = shared.ready.lock().unwrap();
    // R4/R9: loop — re-check the predicate after every wake.
    // Handles spurious wakeups and the lost-wakeup race (R5).
    while !*ready {
        // R6: wait atomically releases the lock while blocked
        // and reacquires it before returning.
        ready = shared.cv.wait(ready).unwrap();
    }
    // R9: we only get here when ready == true.
    // R10: exactly one line of output, from the waiter.
    println!("DONE ready={}", *ready);
}

fn notifier(shared: &Shared) {
    // R3: set the flag while holding the lock.
    let mut ready = shared.ready.lock().unwrap();
    *ready = true;
    // R3: signal the condition variable, then release the lock.
    shared.cv.notify_one();
    drop(ready);
}

fn main() {
    // R2: one lock (m) guarding one boolean (ready), plus one cv,
    // shared by both roles.
    let shared = Arc::new(Shared {
        ready: Mutex::new(false),
        cv: Condvar::new(),
    });

    // R1: spawn waiter and notifier so they run concurrently.
    let s_waiter = Arc::clone(&shared);
    let s_notifier = Arc::clone(&shared);

    let waiter_handle = thread::spawn(move || waiter(&s_waiter));
    let notifier_handle = thread::spawn(move || notifier(&s_notifier));

    // R7: join both roles; every interleaving terminates.
    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();
}
