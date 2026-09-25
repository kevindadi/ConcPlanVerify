use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let m = Arc::new((Mutex::new(false), Condvar::new()));
    let m_waiter = Arc::clone(&m);
    let m_notifier = Arc::clone(&m);

    let waiter = thread::spawn(move || {
        let (lock, cv) = &*m_waiter;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
    });

    let notifier = thread::spawn(move || {
        let (lock, cv) = &*m_notifier;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cv.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _cv) = &*m;
    let ready = lock.lock().unwrap();
    println!("DONE ready={}", *ready);
}
