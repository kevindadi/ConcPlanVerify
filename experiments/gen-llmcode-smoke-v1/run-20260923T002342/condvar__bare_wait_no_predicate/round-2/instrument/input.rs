use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    m: Mutex<bool>,
    cv: Condvar,
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(false),
        cv: Condvar::new(),
    });

    let shared_waiter = Arc::clone(&shared);
    let waiter = thread::spawn(move || {
        let mut ready = shared_waiter.m.lock().unwrap();
        while !*ready {
            ready = shared_waiter.cv.wait(ready).unwrap();
        }
        drop(ready);
    });

    let shared_notifier = Arc::clone(&shared);
    let notifier = thread::spawn(move || {
        let mut ready = shared_notifier.m.lock().unwrap();
        *ready = true;
        shared_notifier.cv.notify_one();
        drop(ready);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    println!("DONE ready=true");
}
