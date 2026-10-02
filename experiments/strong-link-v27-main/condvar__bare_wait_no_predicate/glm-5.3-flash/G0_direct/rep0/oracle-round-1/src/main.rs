mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: m (lock), cv (condition variable), ready (shared variable).
    let m = Arc::new(Mutex::new_named("m_mutex0#181", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#225"));

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let waiter = cir_trace::spawn("waiter#323", move || {
        // R4: check the flag first, wait only while the flag is false,
        // and re-check the flag after each wake.
        // R6: the lock is released while blocked (Condvar::wait does this).
        // R9: do not pass the wait until the flag is true.
        let mut guard = waiter_m.lock().unwrap();
        while !*guard {
            guard = waiter_cv.wait(guard).unwrap();
        }
    });

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let notifier = cir_trace::spawn("notifier#846", move || {
        // R3: set the flag to true while holding the lock, signal the
        // condition variable, then release the lock.
        let mut guard = notifier_m.lock().unwrap();
        *guard = true;
        notifier_cv.notify_one();
        drop(guard);
    });

    // R1/R7: both roles run at the same time and must both finish.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // R8/R10: the flag is true in every schedule; print the exact line.
    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
