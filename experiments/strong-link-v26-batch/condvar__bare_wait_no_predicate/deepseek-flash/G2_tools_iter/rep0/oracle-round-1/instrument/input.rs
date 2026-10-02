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

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(State { ready: false }),
        cv: Condvar::new(),
    });

    let waiter_shared = Arc::clone(&shared);
    let waiter_handle = thread::spawn(move || waiter(waiter_shared));

    let notifier_shared = Arc::clone(&shared);
    let notifier_handle = thread::spawn(move || notifier(notifier_shared));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let state = shared.m.lock().unwrap();
    println!("DONE ready={}", state.ready);
}
