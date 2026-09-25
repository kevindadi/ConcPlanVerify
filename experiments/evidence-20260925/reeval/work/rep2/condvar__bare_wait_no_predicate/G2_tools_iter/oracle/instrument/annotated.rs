mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new((Mutex::new_named("m_mutex0", false), Condvar::new_named("m_condvar0")));
    let m2 = Arc::clone(&m);

    let waiter = cir_trace::spawn("waiter", move || {
        let (lock, cv) = &*m2;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        // flag is true here
    });

    let notifier = cir_trace::spawn("notifier", move || {
        let (lock, cv) = &*m;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cv.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _cv) = &*Arc::new((Mutex::new_named("res_mutex0", false), Condvar::new_named("res_condvar0")));
    let _ = lock;
    println!("DONE ready=true");
 cir_trace::finish();}
