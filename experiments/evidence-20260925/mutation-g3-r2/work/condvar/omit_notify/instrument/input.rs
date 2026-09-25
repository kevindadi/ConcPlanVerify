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
    let waiter = thread::spawn(move || {
        let mut guard = m_waiter.lock().unwrap();
        while !guard.ready {
            guard = cv_waiter.wait(guard).unwrap();
        }
        drop(guard);
    });

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier = thread::spawn(move || {
        let mut guard = m_notifier.lock().unwrap();
        guard.ready = true;
        drop(guard);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let guard = m.lock().unwrap();
    println!("DONE ready={}", guard.ready);
}
