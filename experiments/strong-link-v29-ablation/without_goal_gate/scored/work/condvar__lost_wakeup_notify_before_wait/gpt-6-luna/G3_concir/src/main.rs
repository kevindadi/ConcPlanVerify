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
    drop(guard);
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    guard.ready = true;
    cv.notify_one();
    drop(guard);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#475", Shared { ready: false }, __cir_obs_Shared));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#537"));

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let waiter_thread = crate::cir_trace::spawn("waiter#642", move || waiter(waiter_m, waiter_cv));

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let notifier_thread = crate::cir_trace::spawn("notifier#797", move || notifier(notifier_m, notifier_cv));

    waiter_thread.join().unwrap();
    notifier_thread.join().unwrap();

    let guard = m.lock().unwrap();
    println!("DONE ready={}", guard.ready);
 crate::cir_trace::finish();}

fn __cir_obs_Shared(v: &Shared, r: &str) { crate::cir_trace::record_value(&format!("{}::ready", r), v.ready as i64); }
