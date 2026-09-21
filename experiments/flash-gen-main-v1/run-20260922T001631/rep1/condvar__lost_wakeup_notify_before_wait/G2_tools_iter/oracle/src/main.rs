mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    ready: Mutex<bool>,
    condvar: Condvar,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        ready: Mutex::new_named("shared_mutex0", false),
        condvar: Condvar::new_named("shared_condvar0"),
    });

    let waiter_shared = Arc::clone(&shared);
    let waiter = cir_trace::spawn("waiter", move || {
        let mut ready = waiter_shared.ready.lock().unwrap();
        while !*ready {
            ready = waiter_shared.condvar.wait(ready).unwrap();
        }
    });

    let notifier_shared = Arc::clone(&shared);
    let notifier = cir_trace::spawn("notifier", move || {
        let mut ready = notifier_shared.ready.lock().unwrap();
        *ready = true;
        notifier_shared.condvar.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = shared.ready.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
