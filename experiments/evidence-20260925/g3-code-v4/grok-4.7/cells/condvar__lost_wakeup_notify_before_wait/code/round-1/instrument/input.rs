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
    let waiter = thread::spawn(move || waiter(m_waiter, cv_waiter));

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier = thread::spawn(move || notifier(m_notifier, cv_notifier));

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = m.lock().unwrap().ready;
    println!("DONE ready={ready}");
}

fn waiter(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    let mut seen = guard.ready;
    while seen == false {
        guard = cv.wait(guard).unwrap();
        seen = guard.ready;
    }
    drop(guard);
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>) {
    let mut guard = m.lock().unwrap();
    guard.ready = true;
    cv.notify_one();
    drop(guard);
}
