mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn waiter(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
    drop(guard);
}

fn notifier(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    *guard = true;
    cv.notify_one();
    drop(guard);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#412", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#456"));

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter_handle = crate::cir_trace::spawn("waiter#561", move || waiter(&m_waiter, &cv_waiter));

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier_handle = crate::cir_trace::spawn("notifier#723", move || notifier(&m_notifier, &cv_notifier));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
 crate::cir_trace::finish();}
