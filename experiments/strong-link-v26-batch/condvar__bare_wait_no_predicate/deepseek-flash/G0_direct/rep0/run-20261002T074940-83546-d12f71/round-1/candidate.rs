use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    ready: bool,
}

fn main() {
    let m = Arc::new(Mutex::new(Shared { ready: false }));
    let cv = Arc::new(Condvar::new());

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter_handle = thread::spawn(move || waiter(m_waiter, cv_waiter));

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier_handle = thread::spawn(move || notifier(m_notifier, cv_notifier));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let shared = m.lock().unwrap();
    println!("DONE ready={}", shared.ready);
}

fn waiter(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut shared = m.lock().unwrap();

    while !shared.ready {
        shared = cv.wait(shared).unwrap();
    }
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut shared = m.lock().unwrap();

    shared.ready = true;
    cv.notify_one();
}
