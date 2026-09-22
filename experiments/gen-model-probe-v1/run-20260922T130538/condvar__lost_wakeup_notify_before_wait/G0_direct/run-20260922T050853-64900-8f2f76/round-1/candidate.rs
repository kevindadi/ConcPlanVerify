use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // One lock guarding one boolean flag, plus one condition variable,
    // shared between the waiter and the notifier.
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    // Waiter role: checks the flag under the lock, waits only while it is
    // false, and re-checks after every wake. The flag is read before any
    // wait, so a signal that arrives before the wait begins is not missed.
    let waiter_shared = Arc::clone(&shared);
    let waiter = thread::spawn(move || {
        let (lock, cvar) = &*waiter_shared;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            // wait() atomically releases the lock while blocked and
            // re-acquires it before returning, then the loop re-checks.
            ready = cvar.wait(ready).unwrap();
        }
        *ready
    });

    // Notifier role: sets the flag to true while holding the lock, then
    // signals the condition variable, then releases the lock.
    let notifier_shared = Arc::clone(&shared);
    let notifier = thread::spawn(move || {
        let (lock, cvar) = &*notifier_shared;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
    });

    // Both roles must finish in every schedule.
    let ready = waiter.join().unwrap();
    notifier.join().unwrap();

    println!("DONE ready={}", ready);
}
