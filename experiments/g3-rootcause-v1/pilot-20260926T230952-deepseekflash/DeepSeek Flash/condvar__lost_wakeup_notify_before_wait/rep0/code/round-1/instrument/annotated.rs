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
        m: Mutex::new_named("shared_mutex0", false),
        cv: Condvar::new_named("shared_condvar0"),
    });

    let w = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("waiter", move || waiter(shared))
    };
    let n = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier", move || notifier(shared))
    };

    w.join().unwrap();
    n.join().unwrap();

    let ready = shared.m.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
