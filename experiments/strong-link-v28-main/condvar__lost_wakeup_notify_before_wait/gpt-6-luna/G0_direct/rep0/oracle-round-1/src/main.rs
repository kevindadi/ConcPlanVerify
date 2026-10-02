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
    let mut guard = shared.m.lock().unwrap();

    while !guard.ready {
        guard = shared.cv.wait(guard).unwrap();
    }
}

fn notifier(shared: Arc<Shared>) {
    let mut guard = shared.m.lock().unwrap();
    guard.ready = true;
    shared.cv.notify_one();
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_observed("m#511", State { ready: false }, __cir_obs_State),
        cv: Condvar::new_named("cv#561"),
    });

    let waiter_shared = Arc::clone(&shared);
    let waiter_thread = cir_trace::spawn("waiter#646", move || waiter(waiter_shared));

    let notifier_thread = cir_trace::spawn("notifier#719", move || notifier(shared));

    waiter_thread.join().unwrap();
    notifier_thread.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}

fn __cir_obs_State(v: &State, r: &str) { cir_trace::record_value(&format!("{}::ready", r), v.ready as i64); }
