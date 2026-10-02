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
    drop(ready);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("m#482", false),
        cv: Condvar::new_named("cv#515"),
    });

    let waiter_thread = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("waiter#607", move || waiter(shared))
    };

    let notifier_thread = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier#731", move || notifier(shared))
    };

    waiter_thread.join().unwrap();
    notifier_thread.join().unwrap();

    let ready = shared.m.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
