mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn main() { cir_trace::init();
    // Shared resources: m (lock), cv (condition variable), ready (shared flag)
    let m = Arc::new(Mutex::new_named("m_mutex0#159", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#203"));

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);

    let waiter = cir_trace::spawn("waiter#302", move || {
        // R4: check flag first, wait only while flag is false, re-check after each wake.
        // R6: does not hold the lock while blocked (Condvar::wait releases it).
        // R9: does not pass the wait until the flag is true.
        let mut ready = waiter_m.lock().unwrap();
        while !*ready {
            ready = waiter_cv.wait(ready).unwrap();
        }
    });

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);

    let notifier = cir_trace::spawn("notifier#805", move || {
        // R3: set flag to true while holding the lock, signal, then release the lock.
        let mut ready = notifier_m.lock().unwrap();
        *ready = true;
        notifier_cv.notify_one();
        drop(ready);
    });

    // R1/R7: both roles run concurrently and main waits for both to finish.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // R10: print exactly this line and exit.
    println!("DONE ready=true");
 cir_trace::finish();}
