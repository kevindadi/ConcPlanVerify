use std::sync::{Arc, Condvar, Mutex};
use std::thread;

#[allow(unused_imports)]
use concir_sync::Semaphore;

fn waiter(m: &Mutex<bool>, cv: &Condvar) {
    let mut ready = m.lock().unwrap();
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
}

fn notifier(m: &Mutex<bool>, cv: &Condvar) {
    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_one();
    drop(ready);
}

fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter = thread::spawn(move || waiter(&m_waiter, &cv_waiter));

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier = thread::spawn(move || notifier(&m_notifier, &cv_notifier));

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = *m.lock().unwrap();
    println!("DONE ready={ready}");
}
