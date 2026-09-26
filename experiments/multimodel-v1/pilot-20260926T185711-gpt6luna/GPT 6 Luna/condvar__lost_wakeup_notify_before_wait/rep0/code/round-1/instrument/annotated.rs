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
    drop(guard);
}

fn notifier(shared: Arc<Shared>) {
    let mut guard = shared.m.lock().unwrap();
    guard.ready = true;
    shared.cv.notify_one();
    drop(guard);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", State { ready: false }),
        cv: Condvar::new_named("shared_condvar0"),
    });

    let waiter_handle = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("waiter", move || waiter(shared))
    };
    let notifier_handle = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier", move || notifier(shared))
    };

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = shared.m.lock().unwrap().ready;
    println!("DONE ready={ready}");
 cir_trace::finish();}
