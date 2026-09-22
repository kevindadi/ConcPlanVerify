mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    m: Mutex<bool>,
    cv: Condvar,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", false),
        cv: Condvar::new_named("shared_condvar0"),
    });

    let shared_waiter = Arc::clone(&shared);
    let waiter = cir_trace::spawn("waiter", move || {
        let mut ready = shared_waiter.m.lock().unwrap();
        while !*ready {
            ready = shared_waiter.cv.wait(ready).unwrap();
        }
        drop(ready);
    });

    let shared_notifier = Arc::clone(&shared);
    let notifier = cir_trace::spawn("notifier", move || {
        let mut ready = shared_notifier.m.lock().unwrap();
        *ready = true;
        shared_notifier.cv.notify_one();
        drop(ready);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
