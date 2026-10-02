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
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut state = m.lock().unwrap();
    state.ready = true;
    cv.notify_one();
    drop(state);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#459", Shared { ready: false }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#521"));

    let waiter_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("waiter#630", move || waiter(m, cv))
    };

    let notifier_thread = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("notifier#777", move || notifier(m, cv))
    };

    waiter_thread.join().unwrap();
    notifier_thread.join().unwrap();

    let state = m.lock().unwrap();
    println!("DONE ready={}", state.ready);
 cir_trace::finish();}
