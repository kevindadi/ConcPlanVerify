mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

#[allow(unused_imports)]
use concir_sync::Semaphore;

fn waiter(m: &Mutex<bool>, cv: &Condvar) {
    let mut ready = m.lock().unwrap();
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
}

fn notifier(m: &Mutex<bool>, cv: &Condvar) {
    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_one();
    drop(ready);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter = cir_trace::spawn("waiter", move || waiter(&m_waiter, &cv_waiter));

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier = cir_trace::spawn("notifier", move || notifier(&m_notifier, &cv_notifier));

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = *m.lock().unwrap();
    println!("DONE ready={ready}");
 cir_trace::finish();}
