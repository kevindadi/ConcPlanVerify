mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared state: the lock `m` guards the boolean flag `ready`;
    // `cv` is the condition variable paired with `m`.
    let shared = Arc::new((Mutex::new_named("shared_mutex0", false), Condvar::new_named("shared_condvar0")));

    // Waiter role: holds the lock, waits on `cv` only while `ready` is
    // false, and re-checks the flag after every wake. The lock is
    // released while blocked in `cv.wait`.
    let waiter_shared = Arc::clone(&shared);
    let waiter = cir_trace::spawn("waiter", move || {
        let (m, cv) = &*waiter_shared;
        let mut ready = m.lock().unwrap();
        while !*ready {
            ready = cv.wait(ready).unwrap();
        }
    });

    // Notifier role: sets `ready` to true while holding the lock,
    // signals the condition variable, then releases the lock.
    let notifier_shared = Arc::clone(&shared);
    let notifier = cir_trace::spawn("notifier", move || {
        let (m, cv) = &*notifier_shared;
        let mut ready = m.lock().unwrap();
        *ready = true;
        cv.notify_one();
        drop(ready);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (m, _) = &*shared;
    let ready = m.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
