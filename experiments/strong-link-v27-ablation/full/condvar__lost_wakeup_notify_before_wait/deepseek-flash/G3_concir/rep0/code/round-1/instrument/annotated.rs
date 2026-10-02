mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Ready {
    ready: bool,
}

struct Shared {
    m: Mutex<Ready>,
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
        m: Mutex::new_named("m#510", Ready { ready: false }),
        cv: Condvar::new_named("cv#560"),
    });

    let w = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("waiter#640", move || waiter(shared))
    };

    let n = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("notifier#750", move || notifier(shared))
    };

    w.join().unwrap();
    n.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
