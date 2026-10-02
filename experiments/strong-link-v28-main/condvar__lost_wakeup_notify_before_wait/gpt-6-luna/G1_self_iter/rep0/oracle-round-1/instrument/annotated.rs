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
    let m = Arc::new(Mutex::new_named("m_mutex0#392", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#436"));

    let waiter_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("waiter#545", move || waiter(m, cv))
    };

    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("notifier#692", move || notifier(m, cv))
    };

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = *m.lock().unwrap();
    println!("DONE ready={ready}");
 cir_trace::finish();}
