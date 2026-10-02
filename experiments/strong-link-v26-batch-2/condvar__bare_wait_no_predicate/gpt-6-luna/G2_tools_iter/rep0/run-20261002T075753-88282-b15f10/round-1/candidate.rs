use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    m: Mutex<bool>,
    cv: Condvar,
}

fn waiter(shared: Arc<Shared>) {
    let mut ready = shared.m.lock().unwrap();

    while !*ready {
        ready = shared.cv.wait(ready).unwrap();
    }
}

fn notifier(shared: Arc<Shared>) {
    let mut ready = shared.m.lock().unwrap();
    *ready = true;
    shared.cv.notify_one();
    drop(ready);
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(false),
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

    let ready = shared.m.lock().unwrap();
    println!("DONE ready={}", *ready);
}
