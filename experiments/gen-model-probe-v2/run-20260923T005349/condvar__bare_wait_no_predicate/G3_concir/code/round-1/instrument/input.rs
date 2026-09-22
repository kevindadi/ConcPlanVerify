use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    m: Mutex<bool>,
    cv: Condvar,
}

fn notifier(shared: &Shared) {
    let mut ready = shared.m.lock().unwrap();
    *ready = true;
    shared.cv.notify_one();
    drop(ready);
}

fn waiter(shared: &Shared) {
    let mut ready = shared.m.lock().unwrap();
    while !*ready {
        ready = shared.cv.wait(ready).unwrap();
    }
    drop(ready);
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(false),
        cv: Condvar::new(),
    });

    thread::scope(|s| {
        let notifier_shared = Arc::clone(&shared);
        s.spawn(move || notifier(&notifier_shared));

        let waiter_shared = Arc::clone(&shared);
        s.spawn(move || waiter(&waiter_shared));
    });

    println!("DONE ready=true");
}
