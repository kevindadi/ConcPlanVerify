use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    let (m, cv) = (Arc::clone(&shared), Arc::clone(&shared));
    let waiter = thread::spawn(move || {
        let (lock, cv) = (&*m.0, &*cv.1);
        let mut guard = lock.lock().unwrap();
        while !*guard {
            guard = cv.wait(guard).unwrap();
        }
        assert!(*guard);
    });

    let notifier = thread::spawn(move || {
        let (lock, cv) = (&*shared.0, &*shared.1);
        let mut guard = lock.lock().unwrap();
        *guard = true;
        cv.notify_one();
        drop(guard);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = *shared.0.lock().unwrap();
    println!("DONE ready={}", ready);
}
