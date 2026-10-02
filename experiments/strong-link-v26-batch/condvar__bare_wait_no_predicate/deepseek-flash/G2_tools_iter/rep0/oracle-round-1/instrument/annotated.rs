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
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("m#510", State { ready: false }),
        cv: Condvar::new_named("cv#560"),
    });

    let waiter_shared = Arc::clone(&shared);
    let waiter_handle = cir_trace::spawn("waiter#645", move || waiter(waiter_shared));

    let notifier_shared = Arc::clone(&shared);
    let notifier_handle = cir_trace::spawn("notifier#765", move || notifier(notifier_shared));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let state = shared.m.lock().unwrap();
    println!("DONE ready={}", state.ready);
 cir_trace::finish();}
