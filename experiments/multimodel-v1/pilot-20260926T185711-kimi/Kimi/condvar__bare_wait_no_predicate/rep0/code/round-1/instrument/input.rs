use std::sync::{Condvar, Mutex};
use std::thread;

fn main() {
    let m = Mutex::new(false);
    let cv = Condvar::new();

    let waiter = || {
        let mut ready = m.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        drop(ready);
    };

    let notifier = || {
        let mut ready = m.lock().unwrap();
        *ready = true;
        cv.notify_one();
        drop(ready);
    };

    thread::scope(|s| {
        s.spawn(waiter);
        s.spawn(notifier);
    });

    let ready = m.lock().unwrap();
    println!("DONE ready={}", *ready);
}
