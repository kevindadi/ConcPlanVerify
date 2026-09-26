use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn waiter(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut ready = m.lock().unwrap();
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
    drop(ready);
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
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
    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);

    let w = thread::spawn(move || waiter(m_waiter, cv_waiter));
    let n = thread::spawn(move || notifier(m_notifier, cv_notifier));

    w.join().unwrap();
    n.join().unwrap();

    let ready = m.lock().unwrap();
    println!("DONE ready={}", *ready);
}
