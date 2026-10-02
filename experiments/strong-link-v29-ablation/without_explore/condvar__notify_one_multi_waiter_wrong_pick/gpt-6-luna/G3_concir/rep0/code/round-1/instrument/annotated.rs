mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct Shared {
    proceed: bool,
    ready: usize,
}

fn w1(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, g_n: Arc<Semaphore>) {
    let _ = (g12, g_n);
    let mut state = m.lock().unwrap();
    state.ready += 1;
    cv.notify_all();
    while !state.proceed {
        state = cv.wait(state).unwrap();
    }
}

fn w2(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, g_n: Arc<Semaphore>) {
    let _ = (g12, g_n);
    let mut state = m.lock().unwrap();
    state.ready += 1;
    cv.notify_all();
    while !state.proceed {
        state = cv.wait(state).unwrap();
    }
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, g_n: Arc<Semaphore>) {
    let _ = (g12, g_n);
    let mut state = m.lock().unwrap();
    while state.ready < 2 {
        state = cv.wait(state).unwrap();
    }
    state.proceed = true;
    cv.notify_all();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#1013", Shared {
        proceed: false,
        ready: 0,
    }, __cir_obs_Shared));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#1108"));
    let g12 = Semaphore::new_named("g12_semaphore0#1141", 0);
    let g_n = Semaphore::new_named("g_n_semaphore0#1174", 0);

    let w1_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let g_n = Arc::clone(&g_n);
        crate::cir_trace::spawn("w1#1351", move || w1(m, cv, g12, g_n))
    };
    let w2_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let g_n = Arc::clone(&g_n);
        crate::cir_trace::spawn("w2#1569", move || w2(m, cv, g12, g_n))
    };
    let notifier_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let g_n = Arc::clone(&g_n);
        crate::cir_trace::spawn("notifier#1793", move || notifier(m, cv, g12, g_n))
    };

    w1_thread.join().unwrap();
    w2_thread.join().unwrap();
    notifier_thread.join().unwrap();

    println!("DONE waiters=0");
 crate::cir_trace::finish();}

fn __cir_obs_Shared(v: &Shared, r: &str) { crate::cir_trace::record_value(&format!("{}::proceed", r), v.proceed as i64); crate::cir_trace::record_value(&format!("{}::ready", r), v.ready as i64); }
