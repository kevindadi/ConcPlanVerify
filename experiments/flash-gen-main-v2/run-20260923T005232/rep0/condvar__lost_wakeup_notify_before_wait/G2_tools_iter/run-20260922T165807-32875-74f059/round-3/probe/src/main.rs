use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);

    let waiter = thread::spawn(move || {
        let mut ready = m_waiter.lock().unwrap();
        while !*ready {
            ready = cv_waiter.wait(ready).unwrap();
        }
    });

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);

    let notifier = thread::spawn(move || {
        let mut ready = m_notifier.lock().unwrap();
        *ready = true;
        cv_notifier.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = m.lock().unwrap();
    println!("DONE ready={}", *ready);
}
