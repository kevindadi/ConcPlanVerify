mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn waiter(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut ready = m.lock().unwrap();

    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_one();
    drop(ready);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#409", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#453"));

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let waiter_handle = cir_trace::spawn("waiter#558", move || waiter(waiter_m, waiter_cv));

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let notifier_handle = cir_trace::spawn("notifier#713", move || notifier(notifier_m, notifier_cv));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = m.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
