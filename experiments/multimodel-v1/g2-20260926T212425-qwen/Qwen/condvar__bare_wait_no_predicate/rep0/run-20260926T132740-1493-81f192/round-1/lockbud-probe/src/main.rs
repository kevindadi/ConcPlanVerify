use std::sync::{Arc, Mutex, Condvar};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter_handle = thread::spawn(move || {
        let mut ready = m_waiter.lock().unwrap();
        while !*ready {
            ready = cv_waiter.wait(ready).unwrap();
        }
    });

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier_handle = thread::spawn(move || {
        let mut ready = m_notifier.lock().unwrap();
        *ready = true;
        cv_notifier.notify_one();
    });

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE ready=true");
}
