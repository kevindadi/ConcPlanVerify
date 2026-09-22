use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // One lock guarding a boolean flag, plus one condition variable,
    // shared between the waiter and the notifier.
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    // Waiter role.
    let waiter_shared = Arc::clone(&shared);
    let waiter = thread::spawn(move || {
        let (lock, cvar) = &*waiter_shared;
        let mut ready = lock.lock().unwrap();
        // Check the flag first; wait only while it is false, and
        // re-check after every wake. Condvar::wait releases the lock
        // while blocked and reacquires it before returning.
        while !*ready {
            ready = cvar.wait(ready).unwrap();
        }
        // The flag is true here in every schedule.
        println!("DONE ready={}", *ready);
    });

    // Notifier role.
    let notifier_shared = Arc::clone(&shared);
    let notifier = thread::spawn(move || {
        let (lock, cvar) = &*notifier_shared;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
        drop(ready); // release the lock after signaling
    });

    waiter.join().unwrap();
    notifier.join().unwrap();
}
