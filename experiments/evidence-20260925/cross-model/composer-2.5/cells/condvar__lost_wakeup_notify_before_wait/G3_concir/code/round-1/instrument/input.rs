use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    ready: bool,
}

fn waiter(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    while !guard.ready {
        guard = cv.wait(guard).unwrap();
    }
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    guard.ready = true;
    cv.notify_one();
}

fn main() {
    let m = Arc::new(Mutex::new(Shared { ready: false }));
    let cv = Arc::new(Condvar::new());

    let m_w = Arc::clone(&m);
    let cv_w = Arc::clone(&cv);
    let waiter_handle = thread::spawn(move || waiter(m_w, cv_w));

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let notifier_handle = thread::spawn(move || notifier(m_n, cv_n));

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE ready=true");
}
