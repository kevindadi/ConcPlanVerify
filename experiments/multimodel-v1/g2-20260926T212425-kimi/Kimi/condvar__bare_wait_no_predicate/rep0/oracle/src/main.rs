mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: the lock `m`, the condition variable `cv`,
    // and the shared boolean flag `ready` guarded by `m`.
    let m = Arc::new(Mutex::new_named("m_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    // Waiter role: checks the flag first, waits only while it is false,
    // and re-checks after every wake. The lock is released while blocked
    // inside `wait` and re-acquired before the flag is re-checked.
    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter = cir_trace::spawn("waiter", move || {
        let mut ready = m_waiter.lock().unwrap();
        while !*ready {
            ready = cv_waiter.wait(ready).unwrap();
        }
        // Flag is guaranteed true here, even if the notifier signaled
        // before the waiter started waiting.
        println!("DONE ready={}", *ready);
    });

    // Notifier role: sets the flag while holding the lock, signals the
    // condition variable, then releases the lock.
    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier = cir_trace::spawn("notifier", move || {
        let mut ready = m_notifier.lock().unwrap();
        *ready = true;
        cv_notifier.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();
 cir_trace::finish();}
