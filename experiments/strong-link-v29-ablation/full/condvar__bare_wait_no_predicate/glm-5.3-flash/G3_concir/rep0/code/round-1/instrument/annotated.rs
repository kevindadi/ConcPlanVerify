mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    ready: bool,
}

fn waiter(m: &Arc<Mutex<Shared>>, cv: &Arc<Condvar>) {
    // s1: mutex_lock m
    let mut guard = m.lock().unwrap();
    // s2: branch on ready == true, s3/s4: condvar_wait loop
    while !guard.ready {
        guard = cv.wait(guard).unwrap();
    }
    // s5: mutex_unlock m
    drop(guard);
    // s6: return
}

fn notifier(m: &Arc<Mutex<Shared>>, cv: &Arc<Condvar>) {
    // s1: mutex_lock m
    let mut guard = m.lock().unwrap();
    // s2: write_shared ready = true
    guard.ready = true;
    // s3: condvar_notify cv
    cv.notify_one();
    // s4: mutex_unlock m
    drop(guard);
    // s5: return
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#742", Shared { ready: false }, __cir_obs_Shared));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#804"));

    let m_w = Arc::clone(&m);
    let cv_w = Arc::clone(&cv);
    let waiter_handle = crate::cir_trace::spawn("waiter#899", move || waiter(&m_w, &cv_w));

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let notifier_handle = crate::cir_trace::spawn("notifier#1032", move || notifier(&m_n, &cv_n));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE ready=true");
 crate::cir_trace::finish();}

fn __cir_obs_Shared(v: &Shared, r: &str) { crate::cir_trace::record_value(&format!("{}::ready", r), v.ready as i64); }
