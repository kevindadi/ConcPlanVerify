mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let m_for_notifier = Arc::clone(&m);
    let cv_for_notifier = Arc::clone(&cv);

    let notifier = cir_trace::spawn("notifier", move || {
        let mut ready = m_for_notifier.lock().unwrap();
        *ready = true;
        cv_for_notifier.notify_one();
        drop(ready);
    });

    let waiter = cir_trace::spawn("waiter", move || {
        let mut ready = m.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        drop(ready);
    });

    notifier.join().unwrap();
    waiter.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
