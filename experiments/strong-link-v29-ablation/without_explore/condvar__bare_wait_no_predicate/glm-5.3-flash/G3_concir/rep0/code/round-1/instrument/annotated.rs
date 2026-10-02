mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn waiter(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    let mut ready = *guard;
    while !ready {
        guard = cv.wait(guard).unwrap();
        ready = *guard;
    }
    drop(guard);
}

fn notifier(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    *guard = true;
    cv.notify_all();
    drop(guard);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#463", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#507"));

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter_handle = crate::cir_trace::spawn("waiter#612", move || waiter(&m_waiter, &cv_waiter));

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier_handle = crate::cir_trace::spawn("notifier#774", move || notifier(&m_notifier, &cv_notifier));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
 crate::cir_trace::finish();}
