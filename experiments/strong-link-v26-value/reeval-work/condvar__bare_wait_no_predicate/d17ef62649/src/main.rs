mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct State {
    ready: bool,
}

fn waiter(m: Arc<Mutex<State>>, cv: Arc<Condvar>) {
    let mut state = m.lock().unwrap();

    while !state.ready {
        state = cv.wait(state).unwrap();
    }
}

fn notifier(m: Arc<Mutex<State>>, cv: Arc<Condvar>) {
    let mut state = m.lock().unwrap();
    state.ready = true;
    cv.notify_one();
    drop(state);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#456", State { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#517"));

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let waiter_handle = cir_trace::spawn("waiter#622", move || waiter(waiter_m, waiter_cv));

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let notifier_handle = cir_trace::spawn("notifier#777", move || notifier(notifier_m, notifier_cv));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let state = m.lock().unwrap();
    println!("DONE ready={}", state.ready);
 cir_trace::finish();}
