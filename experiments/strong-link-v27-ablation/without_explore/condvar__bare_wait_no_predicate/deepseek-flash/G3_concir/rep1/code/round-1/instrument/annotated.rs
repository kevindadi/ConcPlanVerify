mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn waiter(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    let mut r = *guard;
    while !r {
        guard = cv.wait(guard).unwrap();
        r = *guard;
    }
    drop(guard);
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    *guard = true;
    cv.notify_one();
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#464", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#508"));

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter_handle = cir_trace::spawn("waiter#613", move || waiter(m_waiter, cv_waiter));

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier_handle = cir_trace::spawn("notifier#768", move || notifier(m_notifier, cv_notifier));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
