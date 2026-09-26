use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // Shared resources: the lock `m`, the condition variable `cv`,
    // and the shared boolean flag `ready` guarded by `m`.
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());

    // Waiter role: checks the flag first, waits only while it is false,
    // and re-checks after every wake. The lock is released while blocked
    // inside `wait` and re-acquired before the flag is re-checked.
    let m_waiter = Arc::clone(&m);
    let cv_waiter = Arc::clone(&cv);
    let waiter = thread::spawn(move || {
        let mut ready = m_waiter.lock().unwrap();
        while !*ready {
            ready = cv_waiter.wait(ready).unwrap();
        }
        // Flag is guaranteed true here, even if the notifier signaled
        // before the waiter started waiting.
        println!("DONE ready={}", *ready);
    });

    // Notifier role: sets the flag while holding the lock, signals the
    // condition variable, then releases the lock.
    let m_notifier = Arc::clone(&m);
    let cv_notifier = Arc::clone(&cv);
    let notifier = thread::spawn(move || {
        let mut ready = m_notifier.lock().unwrap();
        *ready = true;
        cv_notifier.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();
}
