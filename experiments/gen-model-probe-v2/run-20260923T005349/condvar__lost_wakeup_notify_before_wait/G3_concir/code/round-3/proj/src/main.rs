mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let h1 = cir_trace::spawn("h1", move || {
        let mut ready = m_waiter.lock().unwrap();
        while !*ready {
            ready = cv_waiter.wait(ready).unwrap();
        }
        drop(ready);
    });

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let h2 = cir_trace::spawn("h2", move || {
        let mut ready = m_notifier.lock().unwrap();
        *ready = true;
        cv_notifier.notify_one();
        drop(ready);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
