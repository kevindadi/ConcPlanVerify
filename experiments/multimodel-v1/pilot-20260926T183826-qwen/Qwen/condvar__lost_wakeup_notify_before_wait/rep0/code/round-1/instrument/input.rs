use std::sync::{Arc, Mutex, Condvar};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(false)); // ready flag guarded by mutex m
    let cv = Arc::new(Condvar::new());

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter_handle = thread::spawn(move || {
        let mut guard = m_waiter.lock().unwrap();
        loop {
            if *guard {
                break;
            }
            guard = cv_waiter.wait(guard).unwrap();
        }
        // mutex_unlock happens when guard goes out of scope
    });

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier_handle = thread::spawn(move || {
        let mut guard = m_notifier.lock().unwrap();
        *guard = true;
        cv_notifier.notify_one();
        // mutex_unlock happens when guard goes out of scope
    });

    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let final_ready = *m.lock().unwrap();
    println!("DONE ready={}", final_ready);
}
