mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", false)); // ready flag guarded by m
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier = cir_trace::spawn("notifier", move || {
        let mut ready = m_notifier.lock().unwrap();
        *ready = true;
        cv_notifier.notify_one();
        drop(ready);
    });

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter = cir_trace::spawn("waiter", move || {
        let mut ready = m_waiter.lock().unwrap();
        while !*ready {
            ready = cv_waiter.wait(ready).unwrap();
        }
        drop(ready);
    });

    notifier.join().unwrap();
    waiter.join().unwrap();

    let ready = *m.lock().unwrap();
    let _ = ready;
    println!("DONE ready=true");
 cir_trace::finish();}
