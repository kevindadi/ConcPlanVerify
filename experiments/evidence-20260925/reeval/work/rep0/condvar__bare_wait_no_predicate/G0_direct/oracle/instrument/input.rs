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
    // flag is true here
}

fn notifier(shared: Arc<Shared>) {
    let mut ready = shared.m.lock().unwrap();
    *ready = true;
    shared.cv.notify_one();
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(false),
        cv: Condvar::new(),
    });

    let shared_w = Arc::clone(&shared);
    let shared_n = Arc::clone(&shared);

    let waiter_handle = thread::spawn(move || waiter(shared_w));
    let notifier_handle = thread::spawn(move || notifier(shared_n));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let ready = shared.m.lock().unwrap();
    println!("DONE ready={}", *ready);
}
