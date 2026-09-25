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
    // flag is true here
}

fn notifier(shared: Arc<Shared>) {
    let mut ready = shared.m.lock().unwrap();
    *ready = true;
    shared.cv.notify_one();
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", false),
        cv: Condvar::new_named("shared_condvar0"),
    });

    let shared_w = Arc::clone(&shared);
    let shared_n = Arc::clone(&shared);

    let waiter_handle = cir_trace::spawn("waiter", move || waiter(shared_w));
    let notifier_handle = cir_trace::spawn("notifier", move || notifier(shared_n));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = shared.m.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
