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
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(false),
        cv: Condvar::new(),
    });

    let w = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || waiter(shared))
    };
    let n = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || notifier(shared))
    };

    w.join().unwrap();
    n.join().unwrap();

    let ready = shared.m.lock().unwrap();
    println!("DONE ready={}", *ready);
}
