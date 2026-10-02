mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: m (lock), cv (condition variable), ready (shared flag)
    let m = Arc::new(Mutex::new_named("m_mutex0#176", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#220"));

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);

    // waiter role
    let waiter = cir_trace::spawn("waiter#338", move || {
        let mut guard = m_waiter.lock().unwrap();
        // Check the flag first; wait only while the flag is false;
        // re-check after each wake. The lock is released while blocked.
        while !*guard {
            guard = cv_waiter.wait(guard).unwrap();
        }
        // Only pass the wait once the flag is true.
        assert!(*guard);
    });

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);

    // notifier role
    let notifier = cir_trace::spawn("notifier#843", move || {
        let mut guard = m_notifier.lock().unwrap();
        // Set the shared flag to true while holding the lock,
        // signal the condition variable, then release the lock.
        *guard = true;
        cv_notifier.notify_one();
        drop(guard);
    });

    // Both roles run at the same time; wait for both to finish.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // The flag is true in every schedule.
    assert!(*m.lock().unwrap());

    println!("DONE ready=true");
 cir_trace::finish();}
