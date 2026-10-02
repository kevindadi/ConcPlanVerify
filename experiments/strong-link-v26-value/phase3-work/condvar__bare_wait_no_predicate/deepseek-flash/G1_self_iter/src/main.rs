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
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#391", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#435"));

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);

    let waiter_handle = cir_trace::spawn("waiter#617", move || waiter(m_waiter, cv_waiter));
    let notifier_handle = cir_trace::spawn("notifier#695", move || notifier(m_notifier, cv_notifier));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = m.lock().unwrap();
    assert!(*ready);
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
