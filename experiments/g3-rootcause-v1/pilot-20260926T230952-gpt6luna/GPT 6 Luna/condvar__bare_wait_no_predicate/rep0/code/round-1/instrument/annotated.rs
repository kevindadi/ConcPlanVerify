mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn waiter(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();

    while !*guard {
        guard = cv.wait(guard).unwrap();
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
    let m = Arc::new(Mutex::new_named("m_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    let waiter_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("waiter", move || waiter(m, cv))
    };

    let notifier_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("notifier", move || notifier(m, cv))
    };

    waiter_thread.join().unwrap();
    notifier_thread.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
