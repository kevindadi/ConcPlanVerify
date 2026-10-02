mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared state: a boolean flag guarded by a mutex, and a condition variable.
    let m = Arc::new(Mutex::new_named("m_mutex0#178", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#222"));

    // Clone the shared resources for each thread
    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);

    // Start the waiter role
    let waiter_handle = cir_trace::spawn("waiter_handle#483", move || {
        // R2, R4, R6: Acquire the lock and check the flag in a loop.
        // The waiter waits on the condition variable only while the flag is false.
        // It re-checks the flag after each wake to handle spurious wakeups and ensure
        // it doesn't proceed until the flag is true (R9).
        // R5: If the notifier signals before the waiter starts waiting, the flag will
        // already be true when the waiter acquires the lock, so it won't wait at all.
        let mut guard = m_waiter.lock().unwrap();
        while !*guard {
            // Wait releases the lock while blocked (R6), and reacquires it upon waking.
            guard = cv_waiter.wait(guard).unwrap();
        }
        // At this point, *guard == true (R9)
    });

    // Start the notifier role
    let notifier_handle = cir_trace::spawn("notifier_handle#1318", move || {
        // R3: Set the shared flag to true while holding the lock, signal the CV, then release.
        let mut guard = m_notifier.lock().unwrap();
        *guard = true;
        cv_notifier.notify_one();
        // Lock is released here when `guard` goes out of scope
    });

    // R7: Both roles must terminate. Join them to ensure completion.
    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    // R10: Print exactly "DONE ready=true" and exit.
    println!("DONE ready=true");
 cir_trace::finish();}
