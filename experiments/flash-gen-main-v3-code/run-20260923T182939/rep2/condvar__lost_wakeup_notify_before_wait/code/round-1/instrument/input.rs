use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    ready: bool,
}

fn main() {
    let m = Arc::new((Mutex::new(Shared { ready: false }), Condvar::new()));

    let m_waiter = Arc::clone(&m);
    let waiter = thread::spawn(move || {
        let (lock, cv) = &*m_waiter;
        let mut guard = lock.lock().unwrap();
        while !guard.ready {
            guard = cv.wait(guard).unwrap();
        }
        drop(guard);
    });

    let m_notifier = Arc::clone(&m);
    let notifier = thread::spawn(move || {
        let (lock, cv) = &*m_notifier;
        let mut guard = lock.lock().unwrap();
        guard.ready = true;
        cv.notify_one();
        drop(guard);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _cv) = &*m;
    let guard = lock.lock().unwrap();
    println!("DONE ready={}", guard.ready);
}
