mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

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
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#422", State { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#483"));

    let waiter_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("waiter#592", move || waiter(m, cv))
    };

    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("notifier#744", move || notifier(m, cv))
    };

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let state = m.lock().unwrap();
    println!("DONE ready={}", state.ready);
 cir_trace::finish();}
