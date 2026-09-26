mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter_handle = cir_trace::spawn("waiter_handle", move || {
        // Function: main::waiter
        let mut guard = m_waiter.lock().unwrap();
        while !*guard {
            guard = cv_waiter.wait(guard).unwrap();
        }
    });

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier_handle = cir_trace::spawn("notifier_handle", move || {
        // Function: main::notifier
        let mut guard = m_notifier.lock().unwrap();
        *guard = true;
        cv_notifier.notify_all();
    });

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
