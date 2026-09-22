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

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    let waiter = thread::spawn(move || {
        let mut ready = waiter_shared.m.lock().unwrap();
        while !*ready {
            ready = waiter_shared.cv.wait(ready).unwrap();
        }
        // ready is true here
    });

    let notifier = thread::spawn(move || {
        let mut ready = notifier_shared.m.lock().unwrap();
        *ready = true;
        notifier_shared.cv.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = shared.m.lock().unwrap();
    println!("DONE ready={}", *ready);
}
