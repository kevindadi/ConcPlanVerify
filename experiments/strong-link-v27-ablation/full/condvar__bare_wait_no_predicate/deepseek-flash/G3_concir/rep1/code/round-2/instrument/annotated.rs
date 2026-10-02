mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn waiter(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    loop {
        if *guard == false {
            guard = cv.wait(guard).unwrap();
        } else {
            break;
        }
    }
    drop(guard);
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let guard = m.lock().unwrap();
    let mut guard = guard;
    *guard = true;
    cv.notify_one();
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#518", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#562"));

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let waiter_handle = cir_trace::spawn("waiter#667", move || waiter(waiter_m, waiter_cv));

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let notifier_handle = cir_trace::spawn("notifier#822", move || notifier(notifier_m, notifier_cv));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
