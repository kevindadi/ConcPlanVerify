use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn waiter(shared: Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;

    let mut ready = m.lock().unwrap();
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
}

fn notifier(shared: Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*shared;

    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_one();
}

fn main() {
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    let waiter_handle = thread::spawn(move || waiter(waiter_shared));
    let notifier_handle = thread::spawn(move || notifier(notifier_shared));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let (m, _cv) = &*shared;
    let ready = m.lock().unwrap();
    println!("DONE ready={}", *ready);
}
