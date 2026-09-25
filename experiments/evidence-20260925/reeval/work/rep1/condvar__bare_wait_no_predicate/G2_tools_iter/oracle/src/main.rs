mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new((Mutex::new_named("m_mutex0", false), Condvar::new_named("m_condvar0")));
    let m_waiter = Arc::clone(&m);
    let m_notifier = Arc::clone(&m);

    let waiter = cir_trace::spawn("waiter", move || {
        let (lock, cv) = &*m_waiter;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
    });

    let notifier = cir_trace::spawn("notifier", move || {
        let (lock, cv) = &*m_notifier;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cv.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _cv) = &*m;
    let ready = lock.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
