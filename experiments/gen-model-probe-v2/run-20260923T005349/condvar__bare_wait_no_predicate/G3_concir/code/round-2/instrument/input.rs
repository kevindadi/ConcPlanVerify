use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(false)); // ready flag guarded by m
    let cv = Arc::new(Condvar::new());

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier = thread::spawn(move || {
        let mut ready = m_notifier.lock().unwrap();
        *ready = true;
        cv_notifier.notify_one();
        drop(ready);
    });

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter = thread::spawn(move || {
        let mut ready = m_waiter.lock().unwrap();
        while !*ready {
            ready = cv_waiter.wait(ready).unwrap();
        }
        drop(ready);
    });

    notifier.join().unwrap();
    waiter.join().unwrap();

    let ready = *m.lock().unwrap();
    let _ = ready;
    println!("DONE ready=true");
}
