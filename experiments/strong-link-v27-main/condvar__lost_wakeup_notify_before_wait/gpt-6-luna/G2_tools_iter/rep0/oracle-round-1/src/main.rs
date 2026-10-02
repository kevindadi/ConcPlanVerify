mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    m: Mutex<bool>,
    cv: Condvar,
}

fn waiter(shared: Arc<Shared>) {
    let mut ready = shared.m.lock().unwrap();

    while !*ready {
        ready = shared.cv.wait(ready).unwrap();
    }
}

fn notifier(shared: Arc<Shared>) {
    let mut ready = shared.m.lock().unwrap();
    *ready = true;
    shared.cv.notify_one();
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("m#465", false),
        cv: Condvar::new_named("cv#498"),
    });

    let waiter_shared = Arc::clone(&shared);
    let waiter_thread = cir_trace::spawn("waiter#583", move || waiter(waiter_shared));

    let notifier_shared = Arc::clone(&shared);
    let notifier_thread = cir_trace::spawn("notifier#703", move || notifier(notifier_shared));

    waiter_thread.join().unwrap();
    notifier_thread.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
