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
        m: Mutex::new_named("m#464", false),
        cv: Condvar::new_named("cv#497"),
    });

    let shared_waiter = Arc::clone(&shared);
    let waiter_handle = cir_trace::spawn("waiter#582", move || waiter(shared_waiter));

    let shared_notifier = Arc::clone(&shared);
    let notifier_handle = cir_trace::spawn("notifier#702", move || notifier(shared_notifier));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = *shared.m.lock().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
