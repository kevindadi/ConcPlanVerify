mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    ready: bool,
}

fn waiter(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    while !guard.ready {
        guard = cv.wait(guard).unwrap();
    }
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    guard.ready = true;
    cv.notify_one();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#441", Shared { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#503"));

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);

    let waiter_handle = cir_trace::spawn("waiter#685", move || waiter(waiter_m, waiter_cv));
    let notifier_handle = cir_trace::spawn("notifier#763", move || notifier(notifier_m, notifier_cv));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let guard = m.lock().unwrap();
    println!("DONE ready={}", guard.ready);
 cir_trace::finish();}
