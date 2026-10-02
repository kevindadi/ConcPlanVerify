mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn waiter(shared: Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;

    let mut ready = m.lock().unwrap();
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
}

fn notifier(shared: Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;

    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_one();
}

fn main() { cir_trace::init();
    let shared = Arc::new((Mutex::new_named("shared_mutex0#451", false), Condvar::new_named("shared_condvar0#472")));

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    let waiter_handle = cir_trace::spawn("waiter#599", move || waiter(waiter_shared));
    let notifier_handle = cir_trace::spawn("notifier#671", move || notifier(notifier_shared));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let (m, _cv) = &*shared;
    let ready = m.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
