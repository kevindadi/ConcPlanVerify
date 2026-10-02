mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared state: a boolean flag guarded by a mutex, and a condition variable
    let ready = Arc::new((Mutex::new_named("ready_mutex0#182", false), Condvar::new_named("ready_condvar0#203")));

    // Clone the shared state for each thread
    let ready_waiter = Arc::clone(&ready);
    let ready_notifier = Arc::clone(&ready);

    // Spawn the waiter thread
    let waiter_handle = cir_trace::spawn("waiter_handle#403", move || {
        let (lock, cvar) = &*ready_waiter;
        let mut guard = lock.lock().unwrap();
        // While holding the lock, check the flag first.
        // Wait on the condition variable only while the flag is false.
        // Re-check the flag after each wake.
        while !*guard {
            guard = cvar.wait(guard).unwrap();
        }
        // The waiter must not pass its wait until the flag is true.
        // At this point, *guard == true
        println!("DONE ready=true");
    });

    // Spawn the notifier thread
    let notifier_handle = cir_trace::spawn("notifier_handle#987", move || {
        let (lock, cvar) = &*ready_notifier;
        let mut guard = lock.lock().unwrap();
        // Set the shared flag to true while holding the lock
        *guard = true;
        // Signal the condition variable
        cvar.notify_all();
        // Release the lock (automatically when guard goes out of scope)
    });

    // Wait for both threads to finish
    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();
 cir_trace::finish();}
