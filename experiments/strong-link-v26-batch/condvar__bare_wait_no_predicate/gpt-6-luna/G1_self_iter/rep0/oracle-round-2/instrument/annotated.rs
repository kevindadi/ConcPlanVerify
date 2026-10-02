mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct State {
    ready: bool,
}

struct Shared {
    m: Mutex<State>,
    cv: Condvar,
}

fn waiter(shared: Arc<Shared>) {
    let mut state = shared.m.lock().unwrap();

    while !state.ready {
        state = shared.cv.wait(state).unwrap();
    }
}

fn notifier(shared: Arc<Shared>) {
    let mut state = shared.m.lock().unwrap();
    state.ready = true;
    shared.cv.notify_one();
    drop(state);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("m#528", State { ready: false }),
        cv: Condvar::new_named("cv#578"),
    });

    let waiter_handle = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("waiter#670", move || waiter(shared))
    };

    let notifier_handle = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier#794", move || notifier(shared))
    };

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let state = shared.m.lock().unwrap();
    assert!(state.ready);
    println!("DONE ready=true");
 cir_trace::finish();}
