use std::sync::{Arc, Condvar, Mutex};
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

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(State { ready: false }),
        cv: Condvar::new(),
    });

    let waiter_handle = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || waiter(shared))
    };
    let notifier_handle = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || notifier(shared))
    };

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = shared.m.lock().unwrap().ready;
    println!("DONE ready={ready}");
}
