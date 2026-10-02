mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn waiter(shared: &Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;
    let mut ready = m.lock().unwrap();
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
}

fn notifier(shared: &Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;
    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_all();
    drop(ready);
}

fn main() { cir_trace::init();
    let shared = Arc::new((Mutex::new_named("shared_mutex0#468", false), Condvar::new_named("shared_condvar0#489")));
    let s2 = Arc::clone(&shared);
    let s3 = Arc::clone(&shared);

    let w = cir_trace::spawn("waiter#579", move || waiter(&s2));
    let n = cir_trace::spawn("notifier#627", move || notifier(&s3));
    w.join().unwrap();
    n.join().unwrap();

    let ready = *shared.0.lock().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
