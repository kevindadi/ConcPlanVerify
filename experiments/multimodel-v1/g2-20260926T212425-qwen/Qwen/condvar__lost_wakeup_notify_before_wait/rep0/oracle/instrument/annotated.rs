mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared state: a mutex guarding the boolean flag, and a condition variable.
    let m = Arc::new(Mutex::new_named("m_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    // Clone handles for threads
    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);

    // Start notifier thread
    let notifier_handle = cir_trace::spawn("notifier_handle", move || {
        // R3: The notifier sets the shared flag to true while holding the lock,
        // signals the condition variable, and then releases the lock.
        let mut ready = m_notifier.lock().unwrap();
        *ready = true;
        // R6: The flag is set to true before the signal is issued.
        cv_notifier.notify_one();
        // Lock is released here when `ready` goes out of scope.
    });

    // Start waiter thread
    let waiter_handle = cir_trace::spawn("waiter_handle", move || {
        // R4 & R7: The waiter checks the flag while holding the lock,
        // waits on the condition variable only while the flag is false,
        // and re-checks the flag after every wake.
        let mut ready = m_waiter.lock().unwrap();
        
        // R5: The waiter must complete even when the notifier's signal happens 
        // before the waiter starts waiting. This is handled by checking the 
        // predicate (the flag) inside the loop. If the notifier already set 
        // it to true, we skip waiting entirely.
        while !*ready {
            // R7: The waiter releases the lock while blocked.
            // `wait` atomically releases the lock and suspends the thread.
            // Upon waking, it reacquires the lock before returning.
            ready = cv_waiter.wait(ready).unwrap();
        }
        
        // At this point, `ready` is guaranteed to be true because we exited 
        // the loop only when `*ready` was true.
        // The lock is still held here, but we are about to finish.
    });

    // Wait for both threads to complete to ensure termination (R8)
    notifier_handle.join().unwrap();
    waiter_handle.join().unwrap();

    // R10: Print exactly "DONE ready=true"
    println!("DONE ready=true");
 cir_trace::finish();}
