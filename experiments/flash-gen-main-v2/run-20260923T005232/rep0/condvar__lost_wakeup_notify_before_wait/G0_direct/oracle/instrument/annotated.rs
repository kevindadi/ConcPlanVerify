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

    let waiter_shared = Arc::clone(&shared);
    let waiter = cir_trace::spawn("waiter", move || {
        let mut ready = waiter_shared.m.lock().unwrap();
        while !*ready {
            ready = waiter_shared.cv.wait(ready).unwrap();
        }
        *ready
    });

    let notifier_shared = Arc::clone(&shared);
    let notifier = cir_trace::spawn("notifier", move || {
        let mut ready = notifier_shared.m.lock().unwrap();
        *ready = true;
        notifier_shared.cv.notify_one();
    });

    let ready = waiter.join().unwrap();
    notifier.join().unwrap();

    println!("DONE ready={}", ready);
 cir_trace::finish();}
