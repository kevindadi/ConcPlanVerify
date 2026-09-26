mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", false)); // ready flag guarded by mutex m
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter_handle = cir_trace::spawn("waiter_handle", move || {
        let mut guard = m_waiter.lock().unwrap();
        loop {
            if *guard {
                break;
            }
            guard = cv_waiter.wait(guard).unwrap();
        }
        // mutex_unlock happens when guard goes out of scope
    });

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier_handle = cir_trace::spawn("notifier_handle", move || {
        let mut guard = m_notifier.lock().unwrap();
        *guard = true;
        cv_notifier.notify_one();
        // mutex_unlock happens when guard goes out of scope
    });

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let final_ready = *m.lock().unwrap();
    println!("DONE ready={}", final_ready);
 cir_trace::finish();}
