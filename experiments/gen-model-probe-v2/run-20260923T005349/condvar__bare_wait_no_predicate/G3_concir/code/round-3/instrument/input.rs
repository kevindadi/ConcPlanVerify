use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    let m_for_notifier = Arc::clone(&m);
    let cv_for_notifier = Arc::clone(&cv);

    let notifier = thread::spawn(move || {
        let mut ready = m_for_notifier.lock().unwrap();
        *ready = true;
        cv_for_notifier.notify_one();
        drop(ready);
    });

    let waiter = thread::spawn(move || {
        let mut ready = m.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
        drop(ready);
    });

    notifier.join().unwrap();
    waiter.join().unwrap();

    println!("DONE ready=true");
}
