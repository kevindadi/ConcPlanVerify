mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: mutex `m` guards variable `ready` (bool), with condvar `cv`.
    let m = Arc::new(Mutex::new_named("m_mutex0#182", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#226"));

    // Role: waiter
    let m_w = Arc::clone(&m);
    let cv_w = Arc::clone(&cv);
    let waiter = cir_trace::spawn("waiter#334", move || {
        // mutex_lock main::m
        let mut guard = m_w.lock().unwrap();
        // branch: ready == false -> wait, else -> unlock
        while !*guard {
            // condvar_wait main::cv / main::m
            guard = cv_w.wait(guard).unwrap();
        }
        // mutex_unlock main::m
        drop(guard);
    });

    // Role: notifier
    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let notifier = cir_trace::spawn("notifier#784", move || {
        // mutex_lock main::m
        let mut guard = m_n.lock().unwrap();
        // write_shared main::ready = true
        *guard = true;
        // condvar_notify main::cv
        cv_n.notify_one();
        // mutex_unlock main::m
        drop(guard);
    });

    // Join every spawned thread.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // After joins, read shared state to print the terminal line.
    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
