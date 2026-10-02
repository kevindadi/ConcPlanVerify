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
}

fn notifier(shared: Arc<Shared>) {
    let mut guard = shared.m.lock().unwrap();
    guard.ready = true;
    shared.cv.notify_one();
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(State { ready: false }),
        cv: Condvar::new(),
    });

    let waiter_shared = Arc::clone(&shared);
    let waiter_thread = thread::spawn(move || waiter(waiter_shared));

    let notifier_thread = thread::spawn(move || notifier(shared));

    waiter_thread.join().unwrap();
    notifier_thread.join().unwrap();

    println!("DONE ready=true");
}
