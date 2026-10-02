mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// R2: one lock (m), one condition variable (cv), guarding the boolean flag (ready).
struct Shared {
    ready: Mutex<bool>, // m guards `ready`
    cv: Condvar,
}

fn waiter(shared: Arc<Shared>) {
    // Acquire the lock.
    let mut ready = shared.ready.lock().unwrap();

    // R4/R5/R9: check the flag first; wait only while it is false;
    // re-check after every wake. `wait` atomically releases the lock
    // while blocked (R6) and reacquires it before returning.
    while !*ready {
        ready = shared.cv.wait(ready).unwrap();
    }

    // Here *ready == true is guaranteed (R9).
    println!("DONE ready=true"); // R10
}

fn notifier(shared: Arc<Shared>) {
    // R3: set the flag to true while holding the lock...
    let mut ready = shared.ready.lock().unwrap();
    *ready = true;

    // ...signal the condition variable while still holding the lock...
    shared.cv.notify_one();

    // ...then release the lock.
    drop(ready);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        ready: Mutex::new_named("ready#1081", false),
        cv: Condvar::new_named("cv#1114"),
    });

    // R1: main starts waiter and notifier; they run concurrently.
    let waiter_handle = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("waiter#1273", move || waiter(shared))
    };
    let notifier_handle = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier#1396", move || notifier(shared))
    };

    // R7: wait for both roles to finish in every schedule.
    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();
 cir_trace::finish();}
