mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#96", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#140"));

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let waiter = cir_trace::spawn("waiter#238", move || {
        let mut ready = waiter_m.lock().unwrap();
        while !*ready {
            ready = waiter_cv.wait(ready).unwrap();
        }
    });

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let notifier = cir_trace::spawn("notifier#502", move || {
        let mut ready = notifier_m.lock().unwrap();
        *ready = true;
        notifier_cv.notify_one();
        drop(ready);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
