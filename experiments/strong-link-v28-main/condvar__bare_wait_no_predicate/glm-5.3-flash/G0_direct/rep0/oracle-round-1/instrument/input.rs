use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // Shared resources: m (lock), cv (condition variable), ready (shared flag)
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);

    // waiter role
    let waiter = thread::spawn(move || {
        let mut guard = m_waiter.lock().unwrap();
        // Check the flag first; wait only while the flag is false;
        // re-check after each wake. The lock is released while blocked.
        while !*guard {
            guard = cv_waiter.wait(guard).unwrap();
        }
        // Only pass the wait once the flag is true.
        assert!(*guard);
    });

    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);

    // notifier role
    let notifier = thread::spawn(move || {
        let mut guard = m_notifier.lock().unwrap();
        // Set the shared flag to true while holding the lock,
        // signal the condition variable, then release the lock.
        *guard = true;
        cv_notifier.notify_one();
        drop(guard);
    });

    // Both roles run at the same time; wait for both to finish.
    waiter.join().unwrap();
    notifier.join().unwrap();

    // The flag is true in every schedule.
    assert!(*m.lock().unwrap());

    println!("DONE ready=true");
}
