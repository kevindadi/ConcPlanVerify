mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    m: Mutex<bool>,
    cv: Condvar,
}

fn notifier(shared: &Shared) {
    let mut ready = shared.m.lock().unwrap();
    *ready = true;
    shared.cv.notify_one();
    drop(ready);
}

fn waiter(shared: &Shared) {
    let mut ready = shared.m.lock().unwrap();
    while !*ready {
        ready = shared.cv.wait(ready).unwrap();
    }
    drop(ready);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", false),
        cv: Condvar::new_named("shared_condvar0"),
    });

    thread::scope(|s| {
        let notifier_shared = Arc::clone(&shared);
        s.spawn(move || notifier(&notifier_shared));

        let waiter_shared = Arc::clone(&shared);
        s.spawn(move || waiter(&waiter_shared));
    });

    println!("DONE ready=true");
 cir_trace::finish();}
