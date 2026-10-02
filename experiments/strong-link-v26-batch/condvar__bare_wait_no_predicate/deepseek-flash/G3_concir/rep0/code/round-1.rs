use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // Shared resources: mutex `m` guards variable `ready` (bool), with condvar `cv`.
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    // Role: waiter
    let m_w = Arc::clone(&m);
    let cv_w = Arc::clone(&cv);
    let waiter = thread::spawn(move || {
        // mutex_lock main::m
        let mut guard = m_w.lock().unwrap();
        // branch: ready == false -> wait, else -> unlock
        while !*guard {
            // condvar_wait main::cv / main::m
            guard = cv_w.wait(guard).unwrap();
        }
        // mutex_unlock main::m
        drop(guard);
    });

    // Role: notifier
    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let notifier = thread::spawn(move || {
        // mutex_lock main::m
        let mut guard = m_n.lock().unwrap();
        // write_shared main::ready = true
        *guard = true;
        // condvar_notify main::cv
        cv_n.notify_one();
        // mutex_unlock main::m
        drop(guard);
    });

    // Join every spawned thread.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // After joins, read shared state to print the terminal line.
    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
}
