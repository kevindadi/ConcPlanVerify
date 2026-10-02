mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    ready: bool,
}

fn waiter(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut state = m.lock().unwrap();
    while !state.ready {
        state = cv.wait(state).unwrap();
    }
    assert!(state.ready);
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut state = m.lock().unwrap();
    state.ready = true;
    cv.notify_one();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#467", Shared { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#529"));

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let waiter_thread = cir_trace::spawn("waiter#634", move || waiter(waiter_m, waiter_cv));

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let notifier_thread = cir_trace::spawn("notifier#789", move || notifier(notifier_m, notifier_cv));

    waiter_thread.join().unwrap();
    notifier_thread.join().unwrap();

    let state = m.lock().unwrap();
    println!("DONE ready={}", state.ready);
 cir_trace::finish();}
